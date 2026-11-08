//! The Vulkan side: instance, device, swapchain, one pipeline, textures, frame submission.
//!
//! The frame is drawn into a persistent image (`Target`), not into the swapchain: uison redraws only
//! the damaged regions, and swapchain images keep no contents between presents. After drawing,
//! the whole target is copied into the acquired swapchain image and presented (FIFO: vsync).
//! One frame in flight: `begin` waits for the previous one, so buffers and textures can be
//! rewritten and freed between frames without tracking which frame still reads them.

use std::ffi::{CStr, c_void};
use std::mem::size_of;
use std::ptr::{null, null_mut};

use crate::vk::*;

static VERT: &[u8] = include_bytes!("../shaders/ui.vert.spv");
static FRAG: &[u8] = include_bytes!("../shaders/ui.frag.spv");

/// SPIR-V words: include_bytes! does not promise 4-byte alignment, and pCode is *const u32.
fn words(spirv: &[u8]) -> Vec<u32> {
    spirv.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect()
}

/// Pipeline 0 draws boxes, lines and textures; the rest are `shader` items.
pub const UI_PIPELINE: usize = 0;

/// Texture 0 is the atlas: text lines and polygon masks.
pub const ATLAS: usize = 0;
pub const ATLAS_SIZE: u32 = 2048;
const MAX_TEXTURES: u32 = 1024;
const TEXTURE_FORMAT: i32 = VK_FORMAT_B8G8R8A8_UNORM;

/// One shape: matches the vertex attributes of `ui.vert`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Inst {
    pub rect: [f32; 4],
    pub color: [f32; 4],
    pub extra: [f32; 4],
    pub clip: [f32; 4],
    pub param: [f32; 4],
}

pub const KIND_FILL: f32 = 0.0;
pub const KIND_STROKE: f32 = 1.0;
pub const KIND_LINE: f32 = 2.0;
pub const KIND_LINE_ROUND: f32 = 3.0;
pub const KIND_TEXTURE: f32 = 4.0;

/// Instances `first..first + count` drawn with pipeline `pipe` and texture `tex`.
pub struct Batch {
    pub pipe: usize,
    pub tex: usize,
    pub first: u32,
    pub count: u32,
}

/// Pixels to put into a texture before drawing: `w`×`h` BGRA at `offset` in `Scene::staging`.
pub struct Upload {
    pub tex: usize,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub offset: usize,
}

/// What one frame draws: built by the canvas, recorded by `Gpu::submit`.
#[derive(Default)]
pub struct Scene {
    pub insts: Vec<Inst>,
    pub batches: Vec<Batch>,
    pub staging: Vec<u8>,
    pub uploads: Vec<Upload>,
}

impl Scene {
    pub fn clear(&mut self) {
        self.insts.clear();
        self.batches.clear();
        self.staging.clear();
        self.uploads.clear();
    }

    /// Draw the frame again from the start: instances and atlas uploads go, image uploads stay
    /// (their textures are cached and will not be uploaded a second time).
    pub fn restart(&mut self) {
        self.insts.clear();
        self.batches.clear();
        self.uploads.retain(|u| u.tex != ATLAS);
    }

    pub fn push(&mut self, tex: usize, inst: Inst) {
        self.push_with(UI_PIPELINE, tex, inst);
    }

    pub fn push_with(&mut self, pipe: usize, tex: usize, inst: Inst) {
        let n = self.insts.len() as u32;
        match self.batches.last_mut() {
            Some(b) if b.pipe == pipe && b.tex == tex => b.count += 1,
            _ => self.batches.push(Batch { pipe, tex, first: n, count: 1 }),
        }
        self.insts.push(inst);
    }

    pub fn upload(&mut self, tex: usize, x: u32, y: u32, w: u32, h: u32, px: &[u8]) {
        // Buffer offsets for image copies must be multiples of the texel size.
        let offset = self.staging.len();
        self.staging.extend_from_slice(px);
        self.uploads.push(Upload { tex, x, y, w, h, offset });
    }
}

struct Buffer {
    buf: VkHandle,
    mem: VkHandle,
    ptr: *mut u8,
    cap: usize,
}

pub struct Texture {
    image: VkHandle,
    mem: VkHandle,
    view: VkHandle,
    set: VkHandle,
    /// The image is in SHADER_READ_ONLY layout (until then its contents are undefined).
    ready: bool,
}

struct Swapchain {
    handle: VkHandle,
    images: Vec<VkHandle>,
    /// A semaphore per image, not per frame: a present may still wait on it.
    done: Vec<VkHandle>,
}

/// Destroys swapchains replaced by a resize on a thread of its own: in some drivers
/// `vkDestroySwapchainKHR` takes 10–20 ms, which would hold up every frame of a live resize.
/// Only the swapchain itself needs external synchronization, so another thread may destroy it.
struct Reaper {
    tx: Option<std::sync::mpsc::Sender<Swapchain>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Reaper {
    fn start(vk: &Vk, device: VkDevice) -> Reaper {
        let (tx, rx) = std::sync::mpsc::channel::<Swapchain>();
        let (destroy_swapchain, destroy_semaphore) = (vk.vkDestroySwapchainKHR, vk.vkDestroySemaphore);
        // The device outlives the thread: `finish` joins it before the device is destroyed.
        let device = device as usize;
        let thread = std::thread::spawn(move || {
            for s in rx {
                unsafe {
                    for sem in s.done {
                        destroy_semaphore(device as VkDevice, sem, null());
                    }
                    destroy_swapchain(device as VkDevice, s.handle, null());
                }
            }
        });
        Reaper { tx: Some(tx), thread: Some(thread) }
    }

    fn retire(&self, s: Swapchain) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(s);
        }
    }

    /// Destroy what is queued and stop.
    fn finish(&mut self) {
        self.tx = None;
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The persistent image the frame is drawn into.
struct Target {
    image: VkHandle,
    mem: VkHandle,
    view: VkHandle,
    framebuffer: VkHandle,
    extent: VkExtent2D,
    /// Contents are undefined: the next frame draws the whole window.
    fresh: bool,
}

pub struct Gpu {
    vk: Vk,
    instance: VkInstance,
    messenger: VkHandle,
    surface: VkHandle,
    phys: VkPhysicalDevice,
    device: VkDevice,
    queue: VkQueue,
    mem: VkPhysicalDeviceMemoryProperties,
    format: VkSurfaceFormatKHR,
    render_pass: VkHandle,
    set_layout: VkHandle,
    layout: VkHandle,
    /// Index `UI_PIPELINE` and the shader items' pipelines; a freed slot is null.
    pipelines: Vec<VkHandle>,
    sampler: VkHandle,
    pool: VkHandle,
    cmd_pool: VkHandle,
    cmd: VkCommandBuffer,
    fence: VkHandle,
    acquired: VkHandle,
    swap: Option<Swapchain>,
    /// Started with the first resize.
    reaper: Option<Reaper>,
    target: Option<Target>,
    /// The swapchain no longer matches the window (resize, out of date).
    stale: bool,
    image: u32,
    instances: Option<Buffer>,
    staging: Option<Buffer>,
    textures: Vec<Option<Texture>>,
    pub device_name: String,
}

/// Validation layer messages (UISON_VULKAN_VALIDATE=1) to stderr.
unsafe extern "system" fn debug_message(
    severity: VkFlags,
    _kind: VkFlags,
    data: *const VkDebugUtilsMessengerCallbackDataEXT,
    _user: *mut c_void,
) -> VkBool32 {
    let msg = unsafe { CStr::from_ptr((*data).pMessage) }.to_string_lossy();
    let level = if severity & VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT != 0 { "error" } else { "warning" };
    log!("vulkan {level}: {msg}");
    VK_FALSE
}

/// The standard two calls: the count, then the items.
fn list<T>(mut call: impl FnMut(*mut u32, *mut T) -> VkResult, name: &str) -> Result<Vec<T>, String> {
    let mut n = 0;
    check(call(&mut n, null_mut()), name)?;
    let mut v = Vec::with_capacity(n as usize);
    check(call(&mut n, v.as_mut_ptr()), name)?;
    // SAFETY: Vulkan filled the first `n` items.
    unsafe { v.set_len(n as usize) };
    Ok(v)
}

fn simple(kind: VkStructureType, flags: VkFlags) -> VkSimpleCreateInfo {
    VkSimpleCreateInfo { sType: kind, pNext: null(), flags }
}

fn barrier(image: VkHandle, old: i32, new: i32, src: VkFlags, dst: VkFlags) -> VkImageMemoryBarrier {
    VkImageMemoryBarrier {
        sType: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
        pNext: null(),
        srcAccessMask: src,
        dstAccessMask: dst,
        oldLayout: old,
        newLayout: new,
        srcQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED,
        dstQueueFamilyIndex: VK_QUEUE_FAMILY_IGNORED,
        image,
        subresourceRange: COLOR_RANGE,
    }
}

impl Gpu {
    pub fn new(hwnd: *mut c_void, hinstance: *mut c_void) -> Result<Gpu, String> {
        let entry = Entry::load()?;
        unsafe {
            // ---- instance (and the validation layer on request) ----
            let validate = std::env::var("UISON_VULKAN_VALIDATE").is_ok_and(|v| v == "1") && {
                let layers = list(|n, p| (entry.vkEnumerateInstanceLayerProperties)(n, p), "vkEnumerateInstanceLayerProperties")?;
                let found = layers.iter().any(|l| CStr::from_ptr(l.layerName.as_ptr()) == c"VK_LAYER_KHRONOS_validation");
                if !found {
                    log!("uison: UISON_VULKAN_VALIDATE: no VK_LAYER_KHRONOS_validation (install the Vulkan SDK)");
                }
                found
            };
            let app = VkApplicationInfo {
                sType: VK_STRUCTURE_TYPE_APPLICATION_INFO,
                pNext: null(),
                pApplicationName: c"uison".as_ptr(),
                applicationVersion: 0,
                pEngineName: c"uison".as_ptr(),
                engineVersion: 0,
                apiVersion: VK_API_VERSION_1_0,
            };
            let layers = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
            let mut exts = vec![c"VK_KHR_surface".as_ptr(), c"VK_KHR_win32_surface".as_ptr()];
            if validate {
                exts.push(c"VK_EXT_debug_utils".as_ptr());
            }
            let messenger_info = VkDebugUtilsMessengerCreateInfoEXT {
                sType: VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                pNext: null(),
                flags: 0,
                messageSeverity: VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT | VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT,
                messageType: VK_DEBUG_UTILS_MESSAGE_TYPE_ALL,
                pfnUserCallback: debug_message,
                pUserData: null_mut(),
            };
            let info = VkInstanceCreateInfo {
                sType: VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
                // The messenger in pNext also catches messages from vkCreateInstance itself.
                pNext: if validate { (&raw const messenger_info).cast() } else { null() },
                flags: 0,
                pApplicationInfo: &app,
                enabledLayerCount: validate as u32,
                ppEnabledLayerNames: layers.as_ptr(),
                enabledExtensionCount: exts.len() as u32,
                ppEnabledExtensionNames: exts.as_ptr(),
            };
            let mut instance = null_mut();
            check((entry.vkCreateInstance)(&info, null(), &mut instance), "vkCreateInstance")?;
            // A failure ends the program (no fallback), the instance is not worth destroying.
            let vk = Vk::load(&entry, instance)?;
            let mut messenger = VK_NULL_HANDLE;
            if validate && let Some(f) = vk.instance_fn(instance, c"vkCreateDebugUtilsMessengerEXT") {
                type Create = unsafe extern "system" fn(VkInstance, *const VkDebugUtilsMessengerCreateInfoEXT, *const c_void, *mut VkHandle) -> VkResult;
                let create: Create = std::mem::transmute::<unsafe extern "system" fn(), Create>(f);
                create(instance, &messenger_info, null(), &mut messenger);
            }

            // From here on errors drop `gpu`, which destroys what has been created so far.
            let mut gpu = Gpu {
                vk,
                instance,
                messenger,
                surface: VK_NULL_HANDLE,
                phys: null_mut(),
                device: null_mut(),
                queue: null_mut(),
                mem: std::mem::zeroed(),
                format: VkSurfaceFormatKHR::default(),
                render_pass: VK_NULL_HANDLE,
                set_layout: VK_NULL_HANDLE,
                layout: VK_NULL_HANDLE,
                pipelines: Vec::new(),
                sampler: VK_NULL_HANDLE,
                pool: VK_NULL_HANDLE,
                cmd_pool: VK_NULL_HANDLE,
                cmd: null_mut(),
                fence: VK_NULL_HANDLE,
                acquired: VK_NULL_HANDLE,
                swap: None,
                reaper: None,
                target: None,
                stale: true,
                image: 0,
                instances: None,
                staging: None,
                textures: Vec::new(),
                device_name: String::new(),
            };
            gpu.init(hwnd, hinstance)?;
            Ok(gpu)
        }
    }

    unsafe fn init(&mut self, hwnd: *mut c_void, hinstance: *mut c_void) -> Result<(), String> {
        unsafe {
            let vk = &self.vk;
            let info = VkWin32SurfaceCreateInfoKHR {
                sType: VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR,
                pNext: null(),
                flags: 0,
                hinstance,
                hwnd,
            };
            check((vk.vkCreateWin32SurfaceKHR)(self.instance, &info, null(), &mut self.surface), "vkCreateWin32SurfaceKHR")?;

            // ---- a GPU with a queue that both draws and presents to this window; discrete preferred ----
            let devices = list(|n, p| (vk.vkEnumeratePhysicalDevices)(self.instance, n, p), "vkEnumeratePhysicalDevices")?;
            let mut chosen: Option<(VkPhysicalDevice, u32, bool, String)> = None;
            for &d in &devices {
                let mut props: VkPhysicalDeviceProperties = std::mem::zeroed();
                (vk.vkGetPhysicalDeviceProperties)(d, &mut props);
                let families = list(
                    |n, p| {
                        (vk.vkGetPhysicalDeviceQueueFamilyProperties)(d, n, p);
                        VK_SUCCESS
                    },
                    "vkGetPhysicalDeviceQueueFamilyProperties",
                )?;
                let family = (0..families.len() as u32).find(|&f| {
                    let mut present = VK_FALSE;
                    (vk.vkGetPhysicalDeviceSurfaceSupportKHR)(d, f, self.surface, &mut present);
                    families[f as usize].queueFlags & VK_QUEUE_GRAPHICS_BIT != 0 && present == VK_TRUE
                });
                let Some(family) = family else { continue };
                let discrete = props.deviceType == VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU;
                if chosen.as_ref().is_none_or(|c| !c.2 && discrete) {
                    let name = CStr::from_ptr(props.deviceName.as_ptr()).to_string_lossy().into_owned();
                    chosen = Some((d, family, discrete, name));
                }
            }
            let (phys, family, _, name) = chosen.ok_or("no GPU can present to this window")?;
            self.phys = phys;
            self.device_name = name;
            (vk.vkGetPhysicalDeviceMemoryProperties)(phys, &mut self.mem);

            let priority = 1.0f32;
            let queue_info = VkDeviceQueueCreateInfo {
                sType: VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                queueFamilyIndex: family,
                queueCount: 1,
                pQueuePriorities: &priority,
            };
            let exts = [c"VK_KHR_swapchain".as_ptr()];
            let device_info = VkDeviceCreateInfo {
                sType: VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                queueCreateInfoCount: 1,
                pQueueCreateInfos: &queue_info,
                enabledLayerCount: 0,
                ppEnabledLayerNames: null(),
                enabledExtensionCount: exts.len() as u32,
                ppEnabledExtensionNames: exts.as_ptr(),
                pEnabledFeatures: null(),
            };
            check((vk.vkCreateDevice)(phys, &device_info, null(), &mut self.device), "vkCreateDevice")?;
            (vk.vkGetDeviceQueue)(self.device, family, 0, &mut self.queue);

            // ---- the surface format: UNORM, so blending happens in sRGB values, as in Direct2D ----
            let formats = list(|n, p| (vk.vkGetPhysicalDeviceSurfaceFormatsKHR)(phys, self.surface, n, p), "vkGetPhysicalDeviceSurfaceFormatsKHR")?;
            self.format = [VK_FORMAT_B8G8R8A8_UNORM, VK_FORMAT_R8G8B8A8_UNORM]
                .iter()
                .find_map(|&f| formats.iter().find(|s| s.format == f).copied())
                .ok_or("the surface has no 8-bit UNORM format")?;

            self.create_render_pass()?;
            self.create_pipeline()?;
            let vk = &self.vk;

            let sampler = VkSamplerCreateInfo {
                sType: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
                pNext: null(),
                flags: 0,
                magFilter: VK_FILTER_LINEAR,
                minFilter: VK_FILTER_LINEAR,
                mipmapMode: 0,
                addressModeU: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                addressModeV: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                addressModeW: VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE,
                mipLodBias: 0.0,
                anisotropyEnable: VK_FALSE,
                maxAnisotropy: 1.0,
                compareEnable: VK_FALSE,
                compareOp: 0,
                minLod: 0.0,
                maxLod: 0.0,
                borderColor: 0,
                unnormalizedCoordinates: VK_FALSE,
            };
            check((vk.vkCreateSampler)(self.device, &sampler, null(), &mut self.sampler), "vkCreateSampler")?;
            let size = VkDescriptorPoolSize { kind: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER, descriptorCount: MAX_TEXTURES };
            let pool = VkDescriptorPoolCreateInfo {
                sType: VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
                pNext: null(),
                flags: VK_DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT,
                maxSets: MAX_TEXTURES,
                poolSizeCount: 1,
                pPoolSizes: &size,
            };
            check((vk.vkCreateDescriptorPool)(self.device, &pool, null(), &mut self.pool), "vkCreateDescriptorPool")?;

            let cmd_pool = VkCommandPoolCreateInfo {
                sType: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
                pNext: null(),
                flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT,
                queueFamilyIndex: family,
            };
            check((vk.vkCreateCommandPool)(self.device, &cmd_pool, null(), &mut self.cmd_pool), "vkCreateCommandPool")?;
            let alloc = VkCommandBufferAllocateInfo {
                sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
                pNext: null(),
                commandPool: self.cmd_pool,
                level: VK_COMMAND_BUFFER_LEVEL_PRIMARY,
                commandBufferCount: 1,
            };
            check((vk.vkAllocateCommandBuffers)(self.device, &alloc, &mut self.cmd), "vkAllocateCommandBuffers")?;
            // Signaled: the first `begin` does not wait.
            let fence = simple(VK_STRUCTURE_TYPE_FENCE_CREATE_INFO, VK_FENCE_CREATE_SIGNALED_BIT);
            check((vk.vkCreateFence)(self.device, &fence, null(), &mut self.fence), "vkCreateFence")?;
            let sem = simple(VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO, 0);
            check((vk.vkCreateSemaphore)(self.device, &sem, null(), &mut self.acquired), "vkCreateSemaphore")?;

            let atlas = self.texture(ATLAS_SIZE, ATLAS_SIZE)?;
            debug_assert_eq!(atlas, ATLAS);
            Ok(())
        }
    }

    unsafe fn create_render_pass(&mut self) -> Result<(), String> {
        // The target keeps its pixels between frames (LOAD) and rests in TRANSFER_SRC
        // layout, ready to be copied into the swapchain.
        let attachment = VkAttachmentDescription {
            flags: 0,
            format: self.format.format,
            samples: VK_SAMPLE_COUNT_1_BIT,
            loadOp: VK_ATTACHMENT_LOAD_OP_LOAD,
            storeOp: VK_ATTACHMENT_STORE_OP_STORE,
            stencilLoadOp: VK_ATTACHMENT_LOAD_OP_DONT_CARE,
            stencilStoreOp: VK_ATTACHMENT_STORE_OP_DONT_CARE,
            initialLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            finalLayout: VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
        };
        let color = VkAttachmentReference { attachment: 0, layout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL };
        let subpass = VkSubpassDescription {
            flags: 0,
            pipelineBindPoint: VK_PIPELINE_BIND_POINT_GRAPHICS,
            inputAttachmentCount: 0,
            pInputAttachments: null(),
            colorAttachmentCount: 1,
            pColorAttachments: &color,
            pResolveAttachments: null(),
            pDepthStencilAttachment: null(),
            preserveAttachmentCount: 0,
            pPreserveAttachments: null(),
        };
        let deps = [
            // Drawing waits for the previous copy out of the target.
            VkSubpassDependency {
                srcSubpass: VK_SUBPASS_EXTERNAL,
                dstSubpass: 0,
                srcStageMask: VK_PIPELINE_STAGE_TRANSFER_BIT,
                dstStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                srcAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                dstAccessMask: VK_ACCESS_COLOR_ATTACHMENT_READ_BIT | VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                dependencyFlags: 0,
            },
            // The copy into the swapchain waits for drawing.
            VkSubpassDependency {
                srcSubpass: 0,
                dstSubpass: VK_SUBPASS_EXTERNAL,
                srcStageMask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
                dstStageMask: VK_PIPELINE_STAGE_TRANSFER_BIT,
                srcAccessMask: VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT,
                dstAccessMask: VK_ACCESS_TRANSFER_READ_BIT,
                dependencyFlags: 0,
            },
        ];
        let info = VkRenderPassCreateInfo {
            sType: VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            pNext: null(),
            flags: 0,
            attachmentCount: 1,
            pAttachments: &attachment,
            subpassCount: 1,
            pSubpasses: &subpass,
            dependencyCount: deps.len() as u32,
            pDependencies: deps.as_ptr(),
        };
        check(unsafe { (self.vk.vkCreateRenderPass)(self.device, &info, null(), &mut self.render_pass) }, "vkCreateRenderPass")
    }

    unsafe fn shader(&self, code: &[u32]) -> Result<VkHandle, String> {
        let info = VkShaderModuleCreateInfo {
            sType: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
            pNext: null(),
            flags: 0,
            codeSize: code.len() * 4,
            pCode: code.as_ptr(),
        };
        let mut module = VK_NULL_HANDLE;
        check(unsafe { (self.vk.vkCreateShaderModule)(self.device, &info, null(), &mut module) }, "vkCreateShaderModule")?;
        Ok(module)
    }

    unsafe fn create_pipeline(&mut self) -> Result<(), String> {
        unsafe {
            let vk = &self.vk;
            let binding = VkDescriptorSetLayoutBinding {
                binding: 0,
                descriptorType: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                descriptorCount: 1,
                stageFlags: VK_SHADER_STAGE_FRAGMENT_BIT,
                pImmutableSamplers: null(),
            };
            let set_info = VkDescriptorSetLayoutCreateInfo {
                sType: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                pNext: null(),
                flags: 0,
                bindingCount: 1,
                pBindings: &binding,
            };
            check((vk.vkCreateDescriptorSetLayout)(self.device, &set_info, null(), &mut self.set_layout), "vkCreateDescriptorSetLayout")?;
            let push = VkPushConstantRange { stageFlags: VK_SHADER_STAGE_VERTEX_BIT, offset: 0, size: 8 };
            let layout_info = VkPipelineLayoutCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
                pNext: null(),
                flags: 0,
                setLayoutCount: 1,
                pSetLayouts: &self.set_layout,
                pushConstantRangeCount: 1,
                pPushConstantRanges: &push,
            };
            check((vk.vkCreatePipelineLayout)(self.device, &layout_info, null(), &mut self.layout), "vkCreatePipelineLayout")?;
            let ui = self.pipeline(&words(FRAG))?;
            self.pipelines.push(ui);
            Ok(())
        }
    }

    /// A pipeline drawing instances with `ui.vert` and the fragment shader `frag`.
    unsafe fn pipeline(&self, frag: &[u32]) -> Result<VkHandle, String> {
        unsafe {
            let vk = &self.vk;
            let vert = self.shader(&words(VERT))?;
            let frag = match self.shader(frag) {
                Ok(f) => f,
                Err(e) => {
                    (vk.vkDestroyShaderModule)(self.device, vert, null());
                    return Err(e);
                }
            };
            let stage = |stage, module| VkPipelineShaderStageCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                stage,
                module,
                pName: c"main".as_ptr(),
                pSpecializationInfo: null(),
            };
            let stages = [stage(VK_SHADER_STAGE_VERTEX_BIT, vert), stage(VK_SHADER_STAGE_FRAGMENT_BIT, frag)];
            let binding = VkVertexInputBindingDescription { binding: 0, stride: size_of::<Inst>() as u32, inputRate: VK_VERTEX_INPUT_RATE_INSTANCE };
            let attrs: [VkVertexInputAttributeDescription; 5] = std::array::from_fn(|i| VkVertexInputAttributeDescription {
                location: i as u32,
                binding: 0,
                format: VK_FORMAT_R32G32B32A32_SFLOAT,
                offset: 16 * i as u32,
            });
            let vertex_input = VkPipelineVertexInputStateCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                vertexBindingDescriptionCount: 1,
                pVertexBindingDescriptions: &binding,
                vertexAttributeDescriptionCount: attrs.len() as u32,
                pVertexAttributeDescriptions: attrs.as_ptr(),
            };
            let assembly = VkPipelineInputAssemblyStateCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                topology: VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
                primitiveRestartEnable: VK_FALSE,
            };
            let viewport = VkPipelineViewportStateCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                viewportCount: 1,
                pViewports: null(),
                scissorCount: 1,
                pScissors: null(),
            };
            let raster = VkPipelineRasterizationStateCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                depthClampEnable: VK_FALSE,
                rasterizerDiscardEnable: VK_FALSE,
                polygonMode: VK_POLYGON_MODE_FILL,
                cullMode: VK_CULL_MODE_NONE,
                frontFace: VK_FRONT_FACE_COUNTER_CLOCKWISE,
                depthBiasEnable: VK_FALSE,
                depthBiasConstantFactor: 0.0,
                depthBiasClamp: 0.0,
                depthBiasSlopeFactor: 0.0,
                lineWidth: 1.0,
            };
            let multisample = VkPipelineMultisampleStateCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                rasterizationSamples: VK_SAMPLE_COUNT_1_BIT,
                sampleShadingEnable: VK_FALSE,
                minSampleShading: 0.0,
                pSampleMask: null(),
                alphaToCoverageEnable: VK_FALSE,
                alphaToOneEnable: VK_FALSE,
            };
            // Premultiplied alpha.
            let blend_attachment = VkPipelineColorBlendAttachmentState {
                blendEnable: VK_TRUE,
                srcColorBlendFactor: VK_BLEND_FACTOR_ONE,
                dstColorBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
                colorBlendOp: VK_BLEND_OP_ADD,
                srcAlphaBlendFactor: VK_BLEND_FACTOR_ONE,
                dstAlphaBlendFactor: VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA,
                alphaBlendOp: VK_BLEND_OP_ADD,
                colorWriteMask: VK_COLOR_COMPONENT_RGBA,
            };
            let blend = VkPipelineColorBlendStateCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                logicOpEnable: VK_FALSE,
                logicOp: 0,
                attachmentCount: 1,
                pAttachments: &blend_attachment,
                blendConstants: [0.0; 4],
            };
            let dynamic = [VK_DYNAMIC_STATE_VIEWPORT, VK_DYNAMIC_STATE_SCISSOR];
            let dynamic = VkPipelineDynamicStateCreateInfo {
                sType: VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                dynamicStateCount: dynamic.len() as u32,
                pDynamicStates: dynamic.as_ptr(),
            };
            let info = VkGraphicsPipelineCreateInfo {
                sType: VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                stageCount: stages.len() as u32,
                pStages: stages.as_ptr(),
                pVertexInputState: &vertex_input,
                pInputAssemblyState: &assembly,
                pTessellationState: null(),
                pViewportState: &viewport,
                pRasterizationState: &raster,
                pMultisampleState: &multisample,
                pDepthStencilState: null(),
                pColorBlendState: &blend,
                pDynamicState: &dynamic,
                layout: self.layout,
                renderPass: self.render_pass,
                subpass: 0,
                basePipelineHandle: VK_NULL_HANDLE,
                basePipelineIndex: -1,
            };
            let mut pipeline = VK_NULL_HANDLE;
            let r = (vk.vkCreateGraphicsPipelines)(self.device, VK_NULL_HANDLE, 1, &info, null(), &mut pipeline);
            (vk.vkDestroyShaderModule)(self.device, vert, null());
            (vk.vkDestroyShaderModule)(self.device, frag, null());
            check(r, "vkCreateGraphicsPipelines")?;
            Ok(pipeline)
        }
    }

    /// A pipeline for the `shader` item: `ui.vert` with a fragment shader from `uison-shader`
    /// (the same layout and blending, so it draws between other instances).
    pub fn shader_pipeline(&mut self, frag: &[u32]) -> Result<usize, String> {
        let p = unsafe { self.pipeline(frag)? };
        Ok(match self.pipelines.iter().position(|&h| h == VK_NULL_HANDLE) {
            Some(i) => {
                self.pipelines[i] = p;
                i
            }
            None => {
                self.pipelines.push(p);
                self.pipelines.len() - 1
            }
        })
    }

    /// Free a shader pipeline: only between frames, like textures.
    pub fn free_pipeline(&mut self, id: usize) {
        if id != UI_PIPELINE && let Some(h) = self.pipelines.get_mut(id) {
            unsafe { (self.vk.vkDestroyPipeline)(self.device, *h, null()) };
            *h = VK_NULL_HANDLE;
        }
    }

    fn memory_type(&self, bits: u32, want: VkFlags) -> Result<u32, String> {
        (0..self.mem.memoryTypeCount)
            .find(|&i| bits & (1 << i) != 0 && self.mem.memoryTypes[i as usize].propertyFlags & want == want)
            .ok_or_else(|| format!("no memory type with flags {want:#x}"))
    }

    unsafe fn allocate(&self, req: &VkMemoryRequirements, want: VkFlags) -> Result<VkHandle, String> {
        let info = VkMemoryAllocateInfo {
            sType: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            pNext: null(),
            allocationSize: req.size,
            memoryTypeIndex: self.memory_type(req.memoryTypeBits, want)?,
        };
        let mut mem = VK_NULL_HANDLE;
        check(unsafe { (self.vk.vkAllocateMemory)(self.device, &info, null(), &mut mem) }, "vkAllocateMemory")?;
        Ok(mem)
    }

    /// A host-visible buffer, mapped for its whole life.
    unsafe fn buffer(&self, cap: usize, usage: VkFlags) -> Result<Buffer, String> {
        unsafe {
            let vk = &self.vk;
            let info = VkBufferCreateInfo {
                sType: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                pNext: null(),
                flags: 0,
                size: cap as u64,
                usage,
                sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                queueFamilyIndexCount: 0,
                pQueueFamilyIndices: null(),
            };
            let mut buf = VK_NULL_HANDLE;
            check((vk.vkCreateBuffer)(self.device, &info, null(), &mut buf), "vkCreateBuffer")?;
            let mut req = VkMemoryRequirements::default();
            (vk.vkGetBufferMemoryRequirements)(self.device, buf, &mut req);
            let mem = match self.allocate(&req, VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT) {
                Ok(m) => m,
                Err(e) => {
                    (vk.vkDestroyBuffer)(self.device, buf, null());
                    return Err(e);
                }
            };
            let b = Buffer { buf, mem, ptr: null_mut(), cap };
            let mut ptr = null_mut();
            let r = (vk.vkBindBufferMemory)(self.device, buf, mem, 0);
            let r = if r == VK_SUCCESS { (vk.vkMapMemory)(self.device, mem, 0, !0, 0, &mut ptr) } else { r };
            if r != VK_SUCCESS {
                self.free_buffer(b);
                return Err(format!("buffer memory: VkResult {r}"));
            }
            Ok(Buffer { ptr: ptr.cast(), ..b })
        }
    }

    unsafe fn free_buffer(&self, b: Buffer) {
        unsafe {
            (self.vk.vkDestroyBuffer)(self.device, b.buf, null());
            (self.vk.vkFreeMemory)(self.device, b.mem, null());
        }
    }

    /// `slot` holds at least `need` bytes.
    unsafe fn ensure(&mut self, slot: fn(&mut Gpu) -> &mut Option<Buffer>, need: usize, usage: VkFlags) -> Result<(), String> {
        if slot(self).as_ref().is_some_and(|b| b.cap >= need) {
            return Ok(());
        }
        if let Some(old) = slot(self).take() {
            unsafe { self.free_buffer(old) };
        }
        let b = unsafe { self.buffer(need.next_power_of_two().max(1 << 16), usage)? };
        *slot(self) = Some(b);
        Ok(())
    }

    unsafe fn image(&self, w: u32, h: u32, format: i32, usage: VkFlags) -> Result<(VkHandle, VkHandle, VkHandle), String> {
        unsafe {
            let vk = &self.vk;
            let info = VkImageCreateInfo {
                sType: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                pNext: null(),
                flags: 0,
                imageType: VK_IMAGE_TYPE_2D,
                format,
                extent: VkExtent3D { width: w, height: h, depth: 1 },
                mipLevels: 1,
                arrayLayers: 1,
                samples: VK_SAMPLE_COUNT_1_BIT,
                tiling: VK_IMAGE_TILING_OPTIMAL,
                usage,
                sharingMode: VK_SHARING_MODE_EXCLUSIVE,
                queueFamilyIndexCount: 0,
                pQueueFamilyIndices: null(),
                initialLayout: VK_IMAGE_LAYOUT_UNDEFINED,
            };
            let mut image = VK_NULL_HANDLE;
            check((vk.vkCreateImage)(self.device, &info, null(), &mut image), "vkCreateImage")?;
            let mut req = VkMemoryRequirements::default();
            (vk.vkGetImageMemoryRequirements)(self.device, image, &mut req);
            let mem = match self.allocate(&req, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT) {
                Ok(m) => m,
                Err(e) => {
                    (vk.vkDestroyImage)(self.device, image, null());
                    return Err(e);
                }
            };
            let view_info = VkImageViewCreateInfo {
                sType: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                pNext: null(),
                flags: 0,
                image,
                viewType: VK_IMAGE_VIEW_TYPE_2D,
                format,
                components: [0; 4],
                subresourceRange: COLOR_RANGE,
            };
            let mut view = VK_NULL_HANDLE;
            let mut r = (vk.vkBindImageMemory)(self.device, image, mem, 0);
            if r == VK_SUCCESS {
                r = (vk.vkCreateImageView)(self.device, &view_info, null(), &mut view);
            }
            if r != VK_SUCCESS {
                (vk.vkDestroyImage)(self.device, image, null());
                (vk.vkFreeMemory)(self.device, mem, null());
                return Err(format!("image: VkResult {r}"));
            }
            Ok((image, mem, view))
        }
    }

    unsafe fn free_image(&self, image: VkHandle, mem: VkHandle, view: VkHandle) {
        unsafe {
            (self.vk.vkDestroyImageView)(self.device, view, null());
            (self.vk.vkDestroyImage)(self.device, image, null());
            (self.vk.vkFreeMemory)(self.device, mem, null());
        }
    }

    /// A `w`×`h` BGRA texture with its descriptor set; contents come with `Scene::upload`.
    pub fn texture(&mut self, w: u32, h: u32) -> Result<usize, String> {
        unsafe {
            let (image, mem, view) = self.image(w, h, TEXTURE_FORMAT, VK_IMAGE_USAGE_TRANSFER_DST_BIT | VK_IMAGE_USAGE_SAMPLED_BIT)?;
            let alloc = VkDescriptorSetAllocateInfo {
                sType: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
                pNext: null(),
                descriptorPool: self.pool,
                descriptorSetCount: 1,
                pSetLayouts: &self.set_layout,
            };
            let mut set = VK_NULL_HANDLE;
            if let Err(e) = check((self.vk.vkAllocateDescriptorSets)(self.device, &alloc, &mut set), "vkAllocateDescriptorSets") {
                self.free_image(image, mem, view);
                return Err(e);
            }
            let image_info = VkDescriptorImageInfo { sampler: self.sampler, imageView: view, imageLayout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL };
            let write = VkWriteDescriptorSet {
                sType: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                pNext: null(),
                dstSet: set,
                dstBinding: 0,
                dstArrayElement: 0,
                descriptorCount: 1,
                descriptorType: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                pImageInfo: &image_info,
                pBufferInfo: null(),
                pTexelBufferView: null(),
            };
            (self.vk.vkUpdateDescriptorSets)(self.device, 1, &write, 0, null());
            let t = Texture { image, mem, view, set, ready: false };
            Ok(match self.textures.iter().position(Option::is_none) {
                Some(i) => {
                    self.textures[i] = Some(t);
                    i
                }
                None => {
                    self.textures.push(Some(t));
                    self.textures.len() - 1
                }
            })
        }
    }

    /// Free a texture: only between frames (after `begin`, before drawing with it again).
    pub fn free_texture(&mut self, id: usize) {
        if let Some(t) = self.textures.get_mut(id).and_then(Option::take) {
            unsafe {
                (self.vk.vkFreeDescriptorSets)(self.device, self.pool, 1, &t.set);
                self.free_image(t.image, t.mem, t.view);
            }
        }
    }

    pub fn resized(&mut self) {
        self.stale = true;
    }

    /// Start a frame: wait for the previous one, fit the swapchain to the window, take an image.
    /// `Ok(None)` — nothing to draw into (the window is minimized). `Some((extent, fresh))`: with `fresh`
    /// the target lost its pixels and the whole window must be drawn. After `Some`, call `submit`.
    pub fn begin(&mut self) -> Result<Option<(VkExtent2D, bool)>, String> {
        unsafe {
            check((self.vk.vkWaitForFences)(self.device, 1, &self.fence, VK_TRUE, u64::MAX), "vkWaitForFences")?;
            for _ in 0..2 {
                if (self.stale || self.swap.is_none()) && !self.recreate()? {
                    return Ok(None);
                }
                let swap = self.swap.as_ref().unwrap();
                let r = (self.vk.vkAcquireNextImageKHR)(self.device, swap.handle, u64::MAX, self.acquired, VK_NULL_HANDLE, &mut self.image);
                match r {
                    VK_SUCCESS | VK_SUBOPTIMAL_KHR => {
                        let t = self.target.as_ref().unwrap();
                        return Ok(Some((t.extent, t.fresh)));
                    }
                    VK_ERROR_OUT_OF_DATE_KHR => self.stale = true,
                    r => return Err(format!("vkAcquireNextImageKHR: VkResult {r}")),
                }
            }
            Ok(None)
        }
    }

    /// A new swapchain for the window's current size (and a new target if the size changed).
    /// `false` — the window has no area.
    unsafe fn recreate(&mut self) -> Result<bool, String> {
        unsafe {
            let vk = &self.vk;
            check((vk.vkDeviceWaitIdle)(self.device), "vkDeviceWaitIdle")?;
            let mut caps = VkSurfaceCapabilitiesKHR::default();
            check((vk.vkGetPhysicalDeviceSurfaceCapabilitiesKHR)(self.phys, self.surface, &mut caps), "vkGetPhysicalDeviceSurfaceCapabilitiesKHR")?;
            // On Windows currentExtent is the client area; 0×0 — minimized.
            let extent = caps.currentExtent;
            if extent.width == 0 || extent.height == 0 {
                return Ok(false);
            }
            if caps.supportedUsageFlags & VK_IMAGE_USAGE_TRANSFER_DST_BIT == 0 {
                return Err("swapchain images cannot be copied into".into());
            }
            let old = self.swap.take();
            let mut count = caps.minImageCount + 1;
            if caps.maxImageCount != 0 {
                count = count.min(caps.maxImageCount);
            }
            let info = VkSwapchainCreateInfoKHR {
                sType: VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
                pNext: null(),
                flags: 0,
                surface: self.surface,
                minImageCount: count,
                imageFormat: self.format.format,
                imageColorSpace: self.format.colorSpace,
                imageExtent: extent,
                imageArrayLayers: 1,
                imageUsage: VK_IMAGE_USAGE_TRANSFER_DST_BIT,
                imageSharingMode: VK_SHARING_MODE_EXCLUSIVE,
                queueFamilyIndexCount: 0,
                pQueueFamilyIndices: null(),
                preTransform: caps.currentTransform,
                compositeAlpha: VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
                presentMode: VK_PRESENT_MODE_FIFO_KHR,
                clipped: VK_TRUE,
                oldSwapchain: old.as_ref().map_or(VK_NULL_HANDLE, |s| s.handle),
            };
            let mut handle = VK_NULL_HANDLE;
            let r = (vk.vkCreateSwapchainKHR)(self.device, &info, null(), &mut handle);
            if let Some(old) = old {
                let device = self.device;
                self.reaper.get_or_insert_with(|| Reaper::start(&self.vk, device)).retire(old);
            }
            check(r, "vkCreateSwapchainKHR")?;
            let images = list(|n, p| (self.vk.vkGetSwapchainImagesKHR)(self.device, handle, n, p), "vkGetSwapchainImagesKHR")?;
            let sem = simple(VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO, 0);
            let mut done = Vec::with_capacity(images.len());
            for _ in &images {
                let mut s = VK_NULL_HANDLE;
                check((self.vk.vkCreateSemaphore)(self.device, &sem, null(), &mut s), "vkCreateSemaphore")?;
                done.push(s);
            }
            self.swap = Some(Swapchain { handle, images, done });
            self.stale = false;
            if self.target.as_ref().is_none_or(|t| t.extent != extent) {
                if let Some(t) = self.target.take() {
                    self.free_target(t);
                }
                self.target = Some(self.create_target(extent)?);
            }
            Ok(true)
        }
    }

    unsafe fn free_swapchain(&self, s: Swapchain) {
        unsafe {
            for sem in s.done {
                (self.vk.vkDestroySemaphore)(self.device, sem, null());
            }
            (self.vk.vkDestroySwapchainKHR)(self.device, s.handle, null());
        }
    }

    unsafe fn create_target(&self, extent: VkExtent2D) -> Result<Target, String> {
        unsafe {
            let usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT;
            let (image, mem, view) = self.image(extent.width, extent.height, self.format.format, usage)?;
            let info = VkFramebufferCreateInfo {
                sType: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
                pNext: null(),
                flags: 0,
                renderPass: self.render_pass,
                attachmentCount: 1,
                pAttachments: &view,
                width: extent.width,
                height: extent.height,
                layers: 1,
            };
            let mut framebuffer = VK_NULL_HANDLE;
            if let Err(e) = check((self.vk.vkCreateFramebuffer)(self.device, &info, null(), &mut framebuffer), "vkCreateFramebuffer") {
                self.free_image(image, mem, view);
                return Err(e);
            }
            Ok(Target { image, mem, view, framebuffer, extent, fresh: true })
        }
    }

    unsafe fn free_target(&self, t: Target) {
        unsafe {
            (self.vk.vkDestroyFramebuffer)(self.device, t.framebuffer, null());
            self.free_image(t.image, t.mem, t.view);
        }
    }

    /// Record and submit the frame, present it. `Ok(true)` — the swapchain went out of date:
    /// the window should be drawn again.
    pub fn submit(&mut self, scene: &Scene) -> Result<bool, String> {
        unsafe {
            self.ensure(|g| &mut g.instances, scene.insts.len().max(1) * size_of::<Inst>(), VK_BUFFER_USAGE_VERTEX_BUFFER_BIT)?;
            self.ensure(|g| &mut g.staging, scene.staging.len().max(1), VK_BUFFER_USAGE_TRANSFER_SRC_BIT)?;
            let insts = self.instances.as_ref().unwrap();
            std::ptr::copy_nonoverlapping(scene.insts.as_ptr().cast::<u8>(), insts.ptr, scene.insts.len() * size_of::<Inst>());
            let staging = self.staging.as_ref().unwrap();
            std::ptr::copy_nonoverlapping(scene.staging.as_ptr(), staging.ptr, scene.staging.len());

            let vk = &self.vk;
            let cmd = self.cmd;
            check((vk.vkResetCommandBuffer)(cmd, 0), "vkResetCommandBuffer")?;
            let begin = VkCommandBufferBeginInfo {
                sType: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                pNext: null(),
                flags: VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
                pInheritanceInfo: null(),
            };
            check((vk.vkBeginCommandBuffer)(cmd, &begin), "vkBeginCommandBuffer")?;
            let image_barrier = |b: &VkImageMemoryBarrier, src: VkFlags, dst: VkFlags| {
                (vk.vkCmdPipelineBarrier)(cmd, src, dst, 0, 0, null(), 0, null(), 1, b);
            };

            let target = self.target.as_mut().unwrap();
            if target.fresh {
                let b = barrier(target.image, VK_IMAGE_LAYOUT_UNDEFINED, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, 0, 0);
                image_barrier(&b, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT);
                target.fresh = false;
            }

            // ---- texture uploads ----
            for (id, tex) in self.textures.iter_mut().enumerate() {
                let Some(tex) = tex else { continue };
                let regions: Vec<VkBufferImageCopy> = scene
                    .uploads
                    .iter()
                    .filter(|u| u.tex == id)
                    .map(|u| VkBufferImageCopy {
                        bufferOffset: u.offset as u64,
                        bufferRowLength: 0,
                        bufferImageHeight: 0,
                        imageSubresource: COLOR_LAYERS,
                        imageOffset: VkOffset3D { x: u.x as i32, y: u.y as i32, z: 0 },
                        imageExtent: VkExtent3D { width: u.w, height: u.h, depth: 1 },
                    })
                    .collect();
                if regions.is_empty() && tex.ready {
                    continue;
                }
                let old = if tex.ready { VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL } else { VK_IMAGE_LAYOUT_UNDEFINED };
                if regions.is_empty() {
                    let b = barrier(tex.image, old, VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL, 0, VK_ACCESS_SHADER_READ_BIT);
                    image_barrier(&b, VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT);
                } else {
                    let b = barrier(tex.image, old, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, VK_ACCESS_SHADER_READ_BIT, VK_ACCESS_TRANSFER_WRITE_BIT);
                    image_barrier(&b, VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT);
                    (vk.vkCmdCopyBufferToImage)(cmd, staging.buf, tex.image, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, regions.len() as u32, regions.as_ptr());
                    let b = barrier(
                        tex.image,
                        VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                        VK_ACCESS_TRANSFER_WRITE_BIT,
                        VK_ACCESS_SHADER_READ_BIT,
                    );
                    image_barrier(&b, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT);
                }
                tex.ready = true;
            }

            // ---- drawing into the target ----
            let extent = target.extent;
            let full = VkRect2D { offset: VkOffset2D::default(), extent };
            let pass = VkRenderPassBeginInfo {
                sType: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
                pNext: null(),
                renderPass: self.render_pass,
                framebuffer: target.framebuffer,
                renderArea: full,
                clearValueCount: 0,
                pClearValues: null(),
            };
            (vk.vkCmdBeginRenderPass)(cmd, &pass, VK_SUBPASS_CONTENTS_INLINE);
            if !scene.insts.is_empty() {
                (vk.vkCmdBindPipeline)(cmd, VK_PIPELINE_BIND_POINT_GRAPHICS, self.pipelines[UI_PIPELINE]);
                let viewport = VkViewport { x: 0.0, y: 0.0, width: extent.width as f32, height: extent.height as f32, minDepth: 0.0, maxDepth: 1.0 };
                (vk.vkCmdSetViewport)(cmd, 0, 1, &viewport);
                (vk.vkCmdSetScissor)(cmd, 0, 1, &full);
                let size = [extent.width as f32, extent.height as f32];
                (vk.vkCmdPushConstants)(cmd, self.layout, VK_SHADER_STAGE_VERTEX_BIT, 0, 8, size.as_ptr().cast());
                let offset = 0u64;
                (vk.vkCmdBindVertexBuffers)(cmd, 0, 1, &insts.buf, &offset);
                let mut bound = usize::MAX;
                let mut pipe = UI_PIPELINE;
                for b in &scene.batches {
                    // Viewport, scissor, push constants and the set survive a switch: same layout, same dynamic state.
                    if b.pipe != pipe {
                        let Some(&p) = self.pipelines.get(b.pipe).filter(|&&p| p != VK_NULL_HANDLE) else { continue };
                        (vk.vkCmdBindPipeline)(cmd, VK_PIPELINE_BIND_POINT_GRAPHICS, p);
                        pipe = b.pipe;
                    }
                    if b.tex != bound {
                        let Some(Some(tex)) = self.textures.get(b.tex) else { continue };
                        (vk.vkCmdBindDescriptorSets)(cmd, VK_PIPELINE_BIND_POINT_GRAPHICS, self.layout, 0, 1, &tex.set, 0, null());
                        bound = b.tex;
                    }
                    (vk.vkCmdDraw)(cmd, 6, b.count, 0, b.first);
                }
            }
            (vk.vkCmdEndRenderPass)(cmd);

            // ---- the target into the swapchain image ----
            let swap = self.swap.as_ref().unwrap();
            let dst = swap.images[self.image as usize];
            let b = barrier(dst, VK_IMAGE_LAYOUT_UNDEFINED, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 0, VK_ACCESS_TRANSFER_WRITE_BIT);
            image_barrier(&b, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_TRANSFER_BIT);
            let copy = VkImageCopy {
                srcSubresource: COLOR_LAYERS,
                srcOffset: VkOffset3D::default(),
                dstSubresource: COLOR_LAYERS,
                dstOffset: VkOffset3D::default(),
                extent: VkExtent3D { width: extent.width, height: extent.height, depth: 1 },
            };
            (vk.vkCmdCopyImage)(cmd, target.image, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, dst, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 1, &copy);
            let b = barrier(dst, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, VK_IMAGE_LAYOUT_PRESENT_SRC_KHR, VK_ACCESS_TRANSFER_WRITE_BIT, 0);
            image_barrier(&b, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT);
            check((vk.vkEndCommandBuffer)(cmd), "vkEndCommandBuffer")?;

            check((vk.vkResetFences)(self.device, 1, &self.fence), "vkResetFences")?;
            let done = swap.done[self.image as usize];
            let wait_stage = VK_PIPELINE_STAGE_TRANSFER_BIT;
            let submit = VkSubmitInfo {
                sType: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                pNext: null(),
                waitSemaphoreCount: 1,
                pWaitSemaphores: &self.acquired,
                pWaitDstStageMask: &wait_stage,
                commandBufferCount: 1,
                pCommandBuffers: &cmd,
                signalSemaphoreCount: 1,
                pSignalSemaphores: &done,
            };
            check((vk.vkQueueSubmit)(self.queue, 1, &submit, self.fence), "vkQueueSubmit")?;
            let present = VkPresentInfoKHR {
                sType: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
                pNext: null(),
                waitSemaphoreCount: 1,
                pWaitSemaphores: &done,
                swapchainCount: 1,
                pSwapchains: &swap.handle,
                pImageIndices: &self.image,
                pResults: null_mut(),
            };
            match (vk.vkQueuePresentKHR)(self.queue, &present) {
                VK_SUCCESS => Ok(false),
                VK_SUBOPTIMAL_KHR | VK_ERROR_OUT_OF_DATE_KHR => {
                    self.stale = true;
                    Ok(true)
                }
                r => Err(format!("vkQueuePresentKHR: VkResult {r}")),
            }
        }
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            let vk = &self.vk;
            if !self.device.is_null() {
                (vk.vkDeviceWaitIdle)(self.device);
                if let Some(r) = &mut self.reaper {
                    r.finish();
                }
                for id in 0..self.textures.len() {
                    self.free_texture(id);
                }
                let vk = &self.vk;
                for b in [self.instances.take(), self.staging.take()].into_iter().flatten() {
                    self.free_buffer(b);
                }
                if let Some(t) = self.target.take() {
                    self.free_target(t);
                }
                if let Some(s) = self.swap.take() {
                    self.free_swapchain(s);
                }
                let d = self.device;
                (vk.vkDestroySemaphore)(d, self.acquired, null());
                (vk.vkDestroyFence)(d, self.fence, null());
                (vk.vkDestroyCommandPool)(d, self.cmd_pool, null());
                (vk.vkDestroyDescriptorPool)(d, self.pool, null());
                (vk.vkDestroySampler)(d, self.sampler, null());
                for &p in &self.pipelines {
                    (vk.vkDestroyPipeline)(d, p, null());
                }
                (vk.vkDestroyPipelineLayout)(d, self.layout, null());
                (vk.vkDestroyDescriptorSetLayout)(d, self.set_layout, null());
                (vk.vkDestroyRenderPass)(d, self.render_pass, null());
                (vk.vkDestroyDevice)(d, null());
            }
            let vk = &self.vk;
            if self.surface != VK_NULL_HANDLE {
                (vk.vkDestroySurfaceKHR)(self.instance, self.surface, null());
            }
            if self.messenger != VK_NULL_HANDLE
                && let Some(f) = vk.instance_fn(self.instance, c"vkDestroyDebugUtilsMessengerEXT")
            {
                type Destroy = unsafe extern "system" fn(VkInstance, VkHandle, *const c_void);
                std::mem::transmute::<unsafe extern "system" fn(), Destroy>(f)(self.instance, self.messenger, null());
            }
            (vk.vkDestroyInstance)(self.instance, null());
        }
    }
}
