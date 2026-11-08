//! Vulkan 1.0 and the WSI extensions by hand — only what the renderer uses. Layouts and values from
//! `vulkan_core.h`. `vulkan-1.dll` is loaded at run time: building needs no Vulkan SDK, and a machine
//! without a Vulkan driver gets an error from `Vk::load` rather than a missing DLL at start.

#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

use std::ffi::{CStr, c_char, c_void};

pub type VkResult = i32;
pub type VkFlags = u32;
pub type VkBool32 = u32;
pub type VkDeviceSize = u64;
pub type VkStructureType = i32;

// Dispatchable handles are pointers, the rest are 64-bit values.
pub type VkInstance = *mut c_void;
pub type VkPhysicalDevice = *mut c_void;
pub type VkDevice = *mut c_void;
pub type VkQueue = *mut c_void;
pub type VkCommandBuffer = *mut c_void;
pub type VkHandle = u64;

pub const VK_NULL_HANDLE: u64 = 0;
pub const VK_SUCCESS: VkResult = 0;
pub const VK_SUBOPTIMAL_KHR: VkResult = 1000001003;
pub const VK_ERROR_OUT_OF_DATE_KHR: VkResult = -1000001004;
pub const VK_FALSE: VkBool32 = 0;
pub const VK_TRUE: VkBool32 = 1;
pub const VK_SUBPASS_EXTERNAL: u32 = !0;
pub const VK_QUEUE_FAMILY_IGNORED: u32 = !0;
pub const VK_API_VERSION_1_0: u32 = 1 << 22;

pub const VK_STRUCTURE_TYPE_APPLICATION_INFO: VkStructureType = 0;
pub const VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO: VkStructureType = 1;
pub const VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO: VkStructureType = 2;
pub const VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO: VkStructureType = 3;
pub const VK_STRUCTURE_TYPE_SUBMIT_INFO: VkStructureType = 4;
pub const VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO: VkStructureType = 5;
pub const VK_STRUCTURE_TYPE_FENCE_CREATE_INFO: VkStructureType = 8;
pub const VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO: VkStructureType = 9;
pub const VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO: VkStructureType = 12;
pub const VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO: VkStructureType = 14;
pub const VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO: VkStructureType = 15;
pub const VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO: VkStructureType = 16;
pub const VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO: VkStructureType = 18;
pub const VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO: VkStructureType = 19;
pub const VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO: VkStructureType = 20;
pub const VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO: VkStructureType = 22;
pub const VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO: VkStructureType = 23;
pub const VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO: VkStructureType = 24;
pub const VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO: VkStructureType = 26;
pub const VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO: VkStructureType = 27;
pub const VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO: VkStructureType = 28;
pub const VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO: VkStructureType = 30;
pub const VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO: VkStructureType = 31;
pub const VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO: VkStructureType = 32;
pub const VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO: VkStructureType = 33;
pub const VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO: VkStructureType = 34;
pub const VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET: VkStructureType = 35;
pub const VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO: VkStructureType = 37;
pub const VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO: VkStructureType = 38;
pub const VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO: VkStructureType = 39;
pub const VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO: VkStructureType = 40;
pub const VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO: VkStructureType = 42;
pub const VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO: VkStructureType = 43;
pub const VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER: VkStructureType = 45;
pub const VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR: VkStructureType = 1000001000;
pub const VK_STRUCTURE_TYPE_PRESENT_INFO_KHR: VkStructureType = 1000001001;
pub const VK_STRUCTURE_TYPE_WIN32_SURFACE_CREATE_INFO_KHR: VkStructureType = 1000009000;
pub const VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT: VkStructureType = 1000128004;

pub const VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU: i32 = 2;
pub const VK_QUEUE_GRAPHICS_BIT: VkFlags = 0x1;

pub const VK_FORMAT_R8G8B8A8_UNORM: i32 = 37;
pub const VK_FORMAT_B8G8R8A8_UNORM: i32 = 44;
pub const VK_FORMAT_R32G32B32A32_SFLOAT: i32 = 109;
pub const VK_PRESENT_MODE_FIFO_KHR: i32 = 2;
pub const VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR: VkFlags = 0x1;

pub const VK_IMAGE_TYPE_2D: i32 = 1;
pub const VK_IMAGE_TILING_OPTIMAL: i32 = 0;
pub const VK_IMAGE_VIEW_TYPE_2D: i32 = 1;
pub const VK_IMAGE_ASPECT_COLOR_BIT: VkFlags = 0x1;
pub const VK_IMAGE_USAGE_TRANSFER_SRC_BIT: VkFlags = 0x1;
pub const VK_IMAGE_USAGE_TRANSFER_DST_BIT: VkFlags = 0x2;
pub const VK_IMAGE_USAGE_SAMPLED_BIT: VkFlags = 0x4;
pub const VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT: VkFlags = 0x10;

pub const VK_IMAGE_LAYOUT_UNDEFINED: i32 = 0;
pub const VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: i32 = 2;
pub const VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL: i32 = 5;
pub const VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL: i32 = 6;
pub const VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL: i32 = 7;
pub const VK_IMAGE_LAYOUT_PRESENT_SRC_KHR: i32 = 1000001002;

pub const VK_SAMPLE_COUNT_1_BIT: VkFlags = 0x1;
pub const VK_ATTACHMENT_LOAD_OP_LOAD: i32 = 0;
pub const VK_ATTACHMENT_LOAD_OP_DONT_CARE: i32 = 2;
pub const VK_ATTACHMENT_STORE_OP_STORE: i32 = 0;
pub const VK_ATTACHMENT_STORE_OP_DONT_CARE: i32 = 1;
pub const VK_SUBPASS_CONTENTS_INLINE: i32 = 0;
pub const VK_PIPELINE_BIND_POINT_GRAPHICS: i32 = 0;

pub const VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST: i32 = 3;
pub const VK_POLYGON_MODE_FILL: i32 = 0;
pub const VK_CULL_MODE_NONE: VkFlags = 0;
pub const VK_FRONT_FACE_COUNTER_CLOCKWISE: i32 = 0;
pub const VK_BLEND_FACTOR_ONE: i32 = 1;
pub const VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA: i32 = 7;
pub const VK_BLEND_OP_ADD: i32 = 0;
pub const VK_COLOR_COMPONENT_RGBA: VkFlags = 0xF;
pub const VK_DYNAMIC_STATE_VIEWPORT: i32 = 0;
pub const VK_DYNAMIC_STATE_SCISSOR: i32 = 1;
pub const VK_VERTEX_INPUT_RATE_INSTANCE: i32 = 1;
pub const VK_SHADER_STAGE_VERTEX_BIT: VkFlags = 0x1;
pub const VK_SHADER_STAGE_FRAGMENT_BIT: VkFlags = 0x10;

pub const VK_FILTER_LINEAR: i32 = 1;
pub const VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE: i32 = 2;
pub const VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER: i32 = 1;
pub const VK_DESCRIPTOR_POOL_CREATE_FREE_DESCRIPTOR_SET_BIT: VkFlags = 0x1;

pub const VK_FENCE_CREATE_SIGNALED_BIT: VkFlags = 0x1;
pub const VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT: VkFlags = 0x2;
pub const VK_COMMAND_BUFFER_LEVEL_PRIMARY: i32 = 0;
pub const VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT: VkFlags = 0x1;

pub const VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT: VkFlags = 0x1;
pub const VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT: VkFlags = 0x2;
pub const VK_MEMORY_PROPERTY_HOST_COHERENT_BIT: VkFlags = 0x4;
pub const VK_BUFFER_USAGE_TRANSFER_SRC_BIT: VkFlags = 0x1;
pub const VK_BUFFER_USAGE_VERTEX_BUFFER_BIT: VkFlags = 0x80;
pub const VK_SHARING_MODE_EXCLUSIVE: i32 = 0;

pub const VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT: VkFlags = 0x1;
pub const VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT: VkFlags = 0x80;
pub const VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT: VkFlags = 0x400;
pub const VK_PIPELINE_STAGE_TRANSFER_BIT: VkFlags = 0x1000;
pub const VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT: VkFlags = 0x2000;
pub const VK_ACCESS_SHADER_READ_BIT: VkFlags = 0x20;
pub const VK_ACCESS_COLOR_ATTACHMENT_READ_BIT: VkFlags = 0x80;
pub const VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT: VkFlags = 0x100;
pub const VK_ACCESS_TRANSFER_READ_BIT: VkFlags = 0x800;
pub const VK_ACCESS_TRANSFER_WRITE_BIT: VkFlags = 0x1000;

pub const VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT: VkFlags = 0x100;
pub const VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT: VkFlags = 0x1000;
pub const VK_DEBUG_UTILS_MESSAGE_TYPE_ALL: VkFlags = 0x7;

#[repr(C)]
pub struct VkApplicationInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub pApplicationName: *const c_char,
    pub applicationVersion: u32,
    pub pEngineName: *const c_char,
    pub engineVersion: u32,
    pub apiVersion: u32,
}

#[repr(C)]
pub struct VkInstanceCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub pApplicationInfo: *const VkApplicationInfo,
    pub enabledLayerCount: u32,
    pub ppEnabledLayerNames: *const *const c_char,
    pub enabledExtensionCount: u32,
    pub ppEnabledExtensionNames: *const *const c_char,
}

#[repr(C)]
pub struct VkLayerProperties {
    pub layerName: [c_char; 256],
    pub specVersion: u32,
    pub implementationVersion: u32,
    pub description: [c_char; 256],
}

#[repr(C)]
pub struct VkPhysicalDeviceProperties {
    pub apiVersion: u32,
    pub driverVersion: u32,
    pub vendorID: u32,
    pub deviceID: u32,
    pub deviceType: i32,
    pub deviceName: [c_char; 256],
    pub pipelineCacheUUID: [u8; 16],
    /// `limits` and `sparseProperties` are not read: room with a margin (they take 528 bytes), align 8.
    pub _rest: [u64; 128],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VkExtent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VkOffset3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[repr(C)]
pub struct VkQueueFamilyProperties {
    pub queueFlags: VkFlags,
    pub queueCount: u32,
    pub timestampValidBits: u32,
    pub minImageTransferGranularity: VkExtent3D,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkMemoryType {
    pub propertyFlags: VkFlags,
    pub heapIndex: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkMemoryHeap {
    pub size: VkDeviceSize,
    pub flags: VkFlags,
}

#[repr(C)]
pub struct VkPhysicalDeviceMemoryProperties {
    pub memoryTypeCount: u32,
    pub memoryTypes: [VkMemoryType; 32],
    pub memoryHeapCount: u32,
    pub memoryHeaps: [VkMemoryHeap; 16],
}

#[repr(C)]
pub struct VkDeviceQueueCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub queueFamilyIndex: u32,
    pub queueCount: u32,
    pub pQueuePriorities: *const f32,
}

#[repr(C)]
pub struct VkDeviceCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub queueCreateInfoCount: u32,
    pub pQueueCreateInfos: *const VkDeviceQueueCreateInfo,
    pub enabledLayerCount: u32,
    pub ppEnabledLayerNames: *const *const c_char,
    pub enabledExtensionCount: u32,
    pub ppEnabledExtensionNames: *const *const c_char,
    pub pEnabledFeatures: *const c_void,
}

#[repr(C)]
pub struct VkBufferCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub size: VkDeviceSize,
    pub usage: VkFlags,
    pub sharingMode: i32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
}

#[repr(C)]
#[derive(Default)]
pub struct VkMemoryRequirements {
    pub size: VkDeviceSize,
    pub alignment: VkDeviceSize,
    pub memoryTypeBits: u32,
}

#[repr(C)]
pub struct VkMemoryAllocateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub allocationSize: VkDeviceSize,
    pub memoryTypeIndex: u32,
}

#[repr(C)]
pub struct VkImageCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub imageType: i32,
    pub format: i32,
    pub extent: VkExtent3D,
    pub mipLevels: u32,
    pub arrayLayers: u32,
    pub samples: VkFlags,
    pub tiling: i32,
    pub usage: VkFlags,
    pub sharingMode: i32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
    pub initialLayout: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageSubresourceRange {
    pub aspectMask: VkFlags,
    pub baseMipLevel: u32,
    pub levelCount: u32,
    pub baseArrayLayer: u32,
    pub layerCount: u32,
}

/// The one color level of a 2D image.
pub const COLOR_RANGE: VkImageSubresourceRange =
    VkImageSubresourceRange { aspectMask: VK_IMAGE_ASPECT_COLOR_BIT, baseMipLevel: 0, levelCount: 1, baseArrayLayer: 0, layerCount: 1 };

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkImageSubresourceLayers {
    pub aspectMask: VkFlags,
    pub mipLevel: u32,
    pub baseArrayLayer: u32,
    pub layerCount: u32,
}

pub const COLOR_LAYERS: VkImageSubresourceLayers =
    VkImageSubresourceLayers { aspectMask: VK_IMAGE_ASPECT_COLOR_BIT, mipLevel: 0, baseArrayLayer: 0, layerCount: 1 };

#[repr(C)]
pub struct VkImageViewCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub image: VkHandle,
    pub viewType: i32,
    pub format: i32,
    /// `VkComponentMapping`: identity is all zeros.
    pub components: [i32; 4],
    pub subresourceRange: VkImageSubresourceRange,
}

#[repr(C)]
pub struct VkSamplerCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub magFilter: i32,
    pub minFilter: i32,
    pub mipmapMode: i32,
    pub addressModeU: i32,
    pub addressModeV: i32,
    pub addressModeW: i32,
    pub mipLodBias: f32,
    pub anisotropyEnable: VkBool32,
    pub maxAnisotropy: f32,
    pub compareEnable: VkBool32,
    pub compareOp: i32,
    pub minLod: f32,
    pub maxLod: f32,
    pub borderColor: i32,
    pub unnormalizedCoordinates: VkBool32,
}

#[repr(C)]
pub struct VkImageMemoryBarrier {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub srcAccessMask: VkFlags,
    pub dstAccessMask: VkFlags,
    pub oldLayout: i32,
    pub newLayout: i32,
    pub srcQueueFamilyIndex: u32,
    pub dstQueueFamilyIndex: u32,
    pub image: VkHandle,
    pub subresourceRange: VkImageSubresourceRange,
}

#[repr(C)]
pub struct VkBufferImageCopy {
    pub bufferOffset: VkDeviceSize,
    pub bufferRowLength: u32,
    pub bufferImageHeight: u32,
    pub imageSubresource: VkImageSubresourceLayers,
    pub imageOffset: VkOffset3D,
    pub imageExtent: VkExtent3D,
}

#[repr(C)]
pub struct VkImageCopy {
    pub srcSubresource: VkImageSubresourceLayers,
    pub srcOffset: VkOffset3D,
    pub dstSubresource: VkImageSubresourceLayers,
    pub dstOffset: VkOffset3D,
    pub extent: VkExtent3D,
}

#[repr(C)]
pub struct VkShaderModuleCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub codeSize: usize,
    pub pCode: *const u32,
}

#[repr(C)]
pub struct VkDescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptorType: i32,
    pub descriptorCount: u32,
    pub stageFlags: VkFlags,
    pub pImmutableSamplers: *const VkHandle,
}

#[repr(C)]
pub struct VkDescriptorSetLayoutCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub bindingCount: u32,
    pub pBindings: *const VkDescriptorSetLayoutBinding,
}

#[repr(C)]
pub struct VkPushConstantRange {
    pub stageFlags: VkFlags,
    pub offset: u32,
    pub size: u32,
}

#[repr(C)]
pub struct VkPipelineLayoutCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub setLayoutCount: u32,
    pub pSetLayouts: *const VkHandle,
    pub pushConstantRangeCount: u32,
    pub pPushConstantRanges: *const VkPushConstantRange,
}

#[repr(C)]
pub struct VkDescriptorPoolSize {
    pub kind: i32,
    pub descriptorCount: u32,
}

#[repr(C)]
pub struct VkDescriptorPoolCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub maxSets: u32,
    pub poolSizeCount: u32,
    pub pPoolSizes: *const VkDescriptorPoolSize,
}

#[repr(C)]
pub struct VkDescriptorSetAllocateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub descriptorPool: VkHandle,
    pub descriptorSetCount: u32,
    pub pSetLayouts: *const VkHandle,
}

#[repr(C)]
pub struct VkDescriptorImageInfo {
    pub sampler: VkHandle,
    pub imageView: VkHandle,
    pub imageLayout: i32,
}

#[repr(C)]
pub struct VkWriteDescriptorSet {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub dstSet: VkHandle,
    pub dstBinding: u32,
    pub dstArrayElement: u32,
    pub descriptorCount: u32,
    pub descriptorType: i32,
    pub pImageInfo: *const VkDescriptorImageInfo,
    pub pBufferInfo: *const c_void,
    pub pTexelBufferView: *const c_void,
}

#[repr(C)]
pub struct VkCommandPoolCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub queueFamilyIndex: u32,
}

#[repr(C)]
pub struct VkCommandBufferAllocateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub commandPool: VkHandle,
    pub level: i32,
    pub commandBufferCount: u32,
}

#[repr(C)]
pub struct VkCommandBufferBeginInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub pInheritanceInfo: *const c_void,
}

/// `VkFenceCreateInfo` and `VkSemaphoreCreateInfo` have the same shape.
#[repr(C)]
pub struct VkSimpleCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
}

#[repr(C)]
pub struct VkSubmitInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub waitSemaphoreCount: u32,
    pub pWaitSemaphores: *const VkHandle,
    pub pWaitDstStageMask: *const VkFlags,
    pub commandBufferCount: u32,
    pub pCommandBuffers: *const VkCommandBuffer,
    pub signalSemaphoreCount: u32,
    pub pSignalSemaphores: *const VkHandle,
}

#[repr(C)]
pub struct VkWin32SurfaceCreateInfoKHR {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub hinstance: *mut c_void,
    pub hwnd: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct VkExtent2D {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VkOffset2D {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VkRect2D {
    pub offset: VkOffset2D,
    pub extent: VkExtent2D,
}

#[repr(C)]
#[derive(Default)]
pub struct VkSurfaceCapabilitiesKHR {
    pub minImageCount: u32,
    pub maxImageCount: u32,
    pub currentExtent: VkExtent2D,
    pub minImageExtent: VkExtent2D,
    pub maxImageExtent: VkExtent2D,
    pub maxImageArrayLayers: u32,
    pub supportedTransforms: VkFlags,
    pub currentTransform: VkFlags,
    pub supportedCompositeAlpha: VkFlags,
    pub supportedUsageFlags: VkFlags,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct VkSurfaceFormatKHR {
    pub format: i32,
    pub colorSpace: i32,
}

#[repr(C)]
pub struct VkSwapchainCreateInfoKHR {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub surface: VkHandle,
    pub minImageCount: u32,
    pub imageFormat: i32,
    pub imageColorSpace: i32,
    pub imageExtent: VkExtent2D,
    pub imageArrayLayers: u32,
    pub imageUsage: VkFlags,
    pub imageSharingMode: i32,
    pub queueFamilyIndexCount: u32,
    pub pQueueFamilyIndices: *const u32,
    pub preTransform: VkFlags,
    pub compositeAlpha: VkFlags,
    pub presentMode: i32,
    pub clipped: VkBool32,
    pub oldSwapchain: VkHandle,
}

#[repr(C)]
pub struct VkPresentInfoKHR {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub waitSemaphoreCount: u32,
    pub pWaitSemaphores: *const VkHandle,
    pub swapchainCount: u32,
    pub pSwapchains: *const VkHandle,
    pub pImageIndices: *const u32,
    pub pResults: *mut VkResult,
}

#[repr(C)]
pub struct VkAttachmentDescription {
    pub flags: VkFlags,
    pub format: i32,
    pub samples: VkFlags,
    pub loadOp: i32,
    pub storeOp: i32,
    pub stencilLoadOp: i32,
    pub stencilStoreOp: i32,
    pub initialLayout: i32,
    pub finalLayout: i32,
}

#[repr(C)]
pub struct VkAttachmentReference {
    pub attachment: u32,
    pub layout: i32,
}

#[repr(C)]
pub struct VkSubpassDescription {
    pub flags: VkFlags,
    pub pipelineBindPoint: i32,
    pub inputAttachmentCount: u32,
    pub pInputAttachments: *const VkAttachmentReference,
    pub colorAttachmentCount: u32,
    pub pColorAttachments: *const VkAttachmentReference,
    pub pResolveAttachments: *const VkAttachmentReference,
    pub pDepthStencilAttachment: *const VkAttachmentReference,
    pub preserveAttachmentCount: u32,
    pub pPreserveAttachments: *const u32,
}

#[repr(C)]
pub struct VkSubpassDependency {
    pub srcSubpass: u32,
    pub dstSubpass: u32,
    pub srcStageMask: VkFlags,
    pub dstStageMask: VkFlags,
    pub srcAccessMask: VkFlags,
    pub dstAccessMask: VkFlags,
    pub dependencyFlags: VkFlags,
}

#[repr(C)]
pub struct VkRenderPassCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub attachmentCount: u32,
    pub pAttachments: *const VkAttachmentDescription,
    pub subpassCount: u32,
    pub pSubpasses: *const VkSubpassDescription,
    pub dependencyCount: u32,
    pub pDependencies: *const VkSubpassDependency,
}

#[repr(C)]
pub struct VkFramebufferCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub renderPass: VkHandle,
    pub attachmentCount: u32,
    pub pAttachments: *const VkHandle,
    pub width: u32,
    pub height: u32,
    pub layers: u32,
}

#[repr(C)]
pub struct VkRenderPassBeginInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub renderPass: VkHandle,
    pub framebuffer: VkHandle,
    pub renderArea: VkRect2D,
    pub clearValueCount: u32,
    pub pClearValues: *const c_void,
}

#[repr(C)]
pub struct VkPipelineShaderStageCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub stage: VkFlags,
    pub module: VkHandle,
    pub pName: *const c_char,
    pub pSpecializationInfo: *const c_void,
}

#[repr(C)]
pub struct VkVertexInputBindingDescription {
    pub binding: u32,
    pub stride: u32,
    pub inputRate: i32,
}

#[repr(C)]
pub struct VkVertexInputAttributeDescription {
    pub location: u32,
    pub binding: u32,
    pub format: i32,
    pub offset: u32,
}

#[repr(C)]
pub struct VkPipelineVertexInputStateCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub vertexBindingDescriptionCount: u32,
    pub pVertexBindingDescriptions: *const VkVertexInputBindingDescription,
    pub vertexAttributeDescriptionCount: u32,
    pub pVertexAttributeDescriptions: *const VkVertexInputAttributeDescription,
}

#[repr(C)]
pub struct VkPipelineInputAssemblyStateCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub topology: i32,
    pub primitiveRestartEnable: VkBool32,
}

#[repr(C)]
pub struct VkViewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub minDepth: f32,
    pub maxDepth: f32,
}

#[repr(C)]
pub struct VkPipelineViewportStateCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub viewportCount: u32,
    pub pViewports: *const VkViewport,
    pub scissorCount: u32,
    pub pScissors: *const VkRect2D,
}

#[repr(C)]
pub struct VkPipelineRasterizationStateCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub depthClampEnable: VkBool32,
    pub rasterizerDiscardEnable: VkBool32,
    pub polygonMode: i32,
    pub cullMode: VkFlags,
    pub frontFace: i32,
    pub depthBiasEnable: VkBool32,
    pub depthBiasConstantFactor: f32,
    pub depthBiasClamp: f32,
    pub depthBiasSlopeFactor: f32,
    pub lineWidth: f32,
}

#[repr(C)]
pub struct VkPipelineMultisampleStateCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub rasterizationSamples: VkFlags,
    pub sampleShadingEnable: VkBool32,
    pub minSampleShading: f32,
    pub pSampleMask: *const u32,
    pub alphaToCoverageEnable: VkBool32,
    pub alphaToOneEnable: VkBool32,
}

#[repr(C)]
pub struct VkPipelineColorBlendAttachmentState {
    pub blendEnable: VkBool32,
    pub srcColorBlendFactor: i32,
    pub dstColorBlendFactor: i32,
    pub colorBlendOp: i32,
    pub srcAlphaBlendFactor: i32,
    pub dstAlphaBlendFactor: i32,
    pub alphaBlendOp: i32,
    pub colorWriteMask: VkFlags,
}

#[repr(C)]
pub struct VkPipelineColorBlendStateCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub logicOpEnable: VkBool32,
    pub logicOp: i32,
    pub attachmentCount: u32,
    pub pAttachments: *const VkPipelineColorBlendAttachmentState,
    pub blendConstants: [f32; 4],
}

#[repr(C)]
pub struct VkPipelineDynamicStateCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub dynamicStateCount: u32,
    pub pDynamicStates: *const i32,
}

#[repr(C)]
pub struct VkGraphicsPipelineCreateInfo {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub stageCount: u32,
    pub pStages: *const VkPipelineShaderStageCreateInfo,
    pub pVertexInputState: *const VkPipelineVertexInputStateCreateInfo,
    pub pInputAssemblyState: *const VkPipelineInputAssemblyStateCreateInfo,
    pub pTessellationState: *const c_void,
    pub pViewportState: *const VkPipelineViewportStateCreateInfo,
    pub pRasterizationState: *const VkPipelineRasterizationStateCreateInfo,
    pub pMultisampleState: *const VkPipelineMultisampleStateCreateInfo,
    pub pDepthStencilState: *const c_void,
    pub pColorBlendState: *const VkPipelineColorBlendStateCreateInfo,
    pub pDynamicState: *const VkPipelineDynamicStateCreateInfo,
    pub layout: VkHandle,
    pub renderPass: VkHandle,
    pub subpass: u32,
    pub basePipelineHandle: VkHandle,
    pub basePipelineIndex: i32,
}

/// The beginning of `VkDebugUtilsMessengerCallbackDataEXT`: only the message is read.
#[repr(C)]
pub struct VkDebugUtilsMessengerCallbackDataEXT {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub pMessageIdName: *const c_char,
    pub messageIdNumber: i32,
    pub pMessage: *const c_char,
}

pub type PFN_vkDebugUtilsMessengerCallbackEXT =
    unsafe extern "system" fn(VkFlags, VkFlags, *const VkDebugUtilsMessengerCallbackDataEXT, *mut c_void) -> VkBool32;

#[repr(C)]
pub struct VkDebugUtilsMessengerCreateInfoEXT {
    pub sType: VkStructureType,
    pub pNext: *const c_void,
    pub flags: VkFlags,
    pub messageSeverity: VkFlags,
    pub messageType: VkFlags,
    pub pfnUserCallback: PFN_vkDebugUtilsMessengerCallbackEXT,
    pub pUserData: *mut c_void,
}

pub type PFN_vkVoidFunction = Option<unsafe extern "system" fn()>;
type GetInstanceProcAddr = unsafe extern "system" fn(VkInstance, *const c_char) -> PFN_vkVoidFunction;

/// A function from the loader as its real signature `F`.
///
/// # Safety
/// `F` is the function's signature: an `unsafe extern "system" fn` from the headers.
unsafe fn cast<F: Copy>(f: unsafe extern "system" fn()) -> F {
    const { assert!(size_of::<F>() == size_of::<unsafe extern "system" fn()>()) };
    unsafe { std::mem::transmute_copy(&f) }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> PFN_vkVoidFunction;
}

/// A table of Vulkan functions: `(vk.vkCmdDraw)(cb, 6, n, 0, 0)`. Every function is taken through
/// `vkGetInstanceProcAddr`; `pAllocator` is always null (the driver's allocator).
macro_rules! functions {
    ($($name:ident: fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        pub struct Vk {
            get: GetInstanceProcAddr,
            $(pub $name: unsafe extern "system" fn($($arg),*) $(-> $ret)?,)*
        }

        impl Vk {
            /// The instance's functions (device ones dispatch through the loader's trampolines).
            ///
            /// # Safety
            /// `instance` was created by `entry`.
            pub unsafe fn load(entry: &Entry, instance: VkInstance) -> Result<Vk, String> {
                let get = entry.get;
                Ok(Vk {
                    get,
                    $($name: match unsafe { get(instance, concat!(stringify!($name), "\0").as_ptr().cast()) } {
                        // SAFETY: the loader returned the function with this name; its signature is from the headers.
                        Some(f) => unsafe { cast(f) },
                        None => return Err(concat!("no ", stringify!($name)).into()),
                    },)*
                })
            }
        }
    };
}

/// `vulkan-1.dll` and the functions available before an instance exists.
pub struct Entry {
    get: GetInstanceProcAddr,
    pub vkCreateInstance: unsafe extern "system" fn(*const VkInstanceCreateInfo, *const c_void, *mut VkInstance) -> VkResult,
    pub vkEnumerateInstanceLayerProperties: unsafe extern "system" fn(*mut u32, *mut VkLayerProperties) -> VkResult,
}

impl Entry {
    pub fn load() -> Result<Entry, String> {
        let name: Vec<u16> = "vulkan-1.dll\0".encode_utf16().collect();
        // The DLL stays loaded until the process ends: the window lives as long as the program.
        let lib = unsafe { LoadLibraryW(name.as_ptr()) };
        if lib.is_null() {
            return Err("vulkan-1.dll not found: no Vulkan driver installed".into());
        }
        let get = unsafe { GetProcAddress(lib, c"vkGetInstanceProcAddr".as_ptr()) }.ok_or("no vkGetInstanceProcAddr in vulkan-1.dll")?;
        // SAFETY: the exported function has this signature.
        let get: GetInstanceProcAddr = unsafe { cast(get) };
        let global = |name: &CStr| unsafe { get(std::ptr::null_mut(), name.as_ptr()) }.ok_or_else(|| format!("no {name:?}"));
        unsafe {
            Ok(Entry {
                get,
                vkCreateInstance: cast(global(c"vkCreateInstance")?),
                vkEnumerateInstanceLayerProperties: cast(global(
                    c"vkEnumerateInstanceLayerProperties",
                )?),
            })
        }
    }
}

impl Vk {
    /// An extension function that the table does not hold.
    pub fn instance_fn(&self, instance: VkInstance, name: &CStr) -> PFN_vkVoidFunction {
        unsafe { (self.get)(instance, name.as_ptr()) }
    }
}

type P = *const c_void;
type H = VkHandle;

functions! {
    vkDestroyInstance: fn(VkInstance, P);
    vkEnumeratePhysicalDevices: fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult;
    vkGetPhysicalDeviceProperties: fn(VkPhysicalDevice, *mut VkPhysicalDeviceProperties);
    vkGetPhysicalDeviceQueueFamilyProperties: fn(VkPhysicalDevice, *mut u32, *mut VkQueueFamilyProperties);
    vkGetPhysicalDeviceMemoryProperties: fn(VkPhysicalDevice, *mut VkPhysicalDeviceMemoryProperties);
    vkCreateDevice: fn(VkPhysicalDevice, *const VkDeviceCreateInfo, P, *mut VkDevice) -> VkResult;
    vkDestroyDevice: fn(VkDevice, P);
    vkDeviceWaitIdle: fn(VkDevice) -> VkResult;
    vkGetDeviceQueue: fn(VkDevice, u32, u32, *mut VkQueue);

    vkCreateWin32SurfaceKHR: fn(VkInstance, *const VkWin32SurfaceCreateInfoKHR, P, *mut H) -> VkResult;
    vkDestroySurfaceKHR: fn(VkInstance, H, P);
    vkGetPhysicalDeviceSurfaceSupportKHR: fn(VkPhysicalDevice, u32, H, *mut VkBool32) -> VkResult;
    vkGetPhysicalDeviceSurfaceCapabilitiesKHR: fn(VkPhysicalDevice, H, *mut VkSurfaceCapabilitiesKHR) -> VkResult;
    vkGetPhysicalDeviceSurfaceFormatsKHR: fn(VkPhysicalDevice, H, *mut u32, *mut VkSurfaceFormatKHR) -> VkResult;
    vkCreateSwapchainKHR: fn(VkDevice, *const VkSwapchainCreateInfoKHR, P, *mut H) -> VkResult;
    vkDestroySwapchainKHR: fn(VkDevice, H, P);
    vkGetSwapchainImagesKHR: fn(VkDevice, H, *mut u32, *mut H) -> VkResult;
    vkAcquireNextImageKHR: fn(VkDevice, H, u64, H, H, *mut u32) -> VkResult;
    vkQueuePresentKHR: fn(VkQueue, *const VkPresentInfoKHR) -> VkResult;

    vkCreateBuffer: fn(VkDevice, *const VkBufferCreateInfo, P, *mut H) -> VkResult;
    vkDestroyBuffer: fn(VkDevice, H, P);
    vkGetBufferMemoryRequirements: fn(VkDevice, H, *mut VkMemoryRequirements);
    vkBindBufferMemory: fn(VkDevice, H, H, VkDeviceSize) -> VkResult;
    vkCreateImage: fn(VkDevice, *const VkImageCreateInfo, P, *mut H) -> VkResult;
    vkDestroyImage: fn(VkDevice, H, P);
    vkGetImageMemoryRequirements: fn(VkDevice, H, *mut VkMemoryRequirements);
    vkBindImageMemory: fn(VkDevice, H, H, VkDeviceSize) -> VkResult;
    vkAllocateMemory: fn(VkDevice, *const VkMemoryAllocateInfo, P, *mut H) -> VkResult;
    vkFreeMemory: fn(VkDevice, H, P);
    vkMapMemory: fn(VkDevice, H, VkDeviceSize, VkDeviceSize, VkFlags, *mut *mut c_void) -> VkResult;
    vkCreateImageView: fn(VkDevice, *const VkImageViewCreateInfo, P, *mut H) -> VkResult;
    vkDestroyImageView: fn(VkDevice, H, P);
    vkCreateSampler: fn(VkDevice, *const VkSamplerCreateInfo, P, *mut H) -> VkResult;
    vkDestroySampler: fn(VkDevice, H, P);

    vkCreateShaderModule: fn(VkDevice, *const VkShaderModuleCreateInfo, P, *mut H) -> VkResult;
    vkDestroyShaderModule: fn(VkDevice, H, P);
    vkCreateDescriptorSetLayout: fn(VkDevice, *const VkDescriptorSetLayoutCreateInfo, P, *mut H) -> VkResult;
    vkDestroyDescriptorSetLayout: fn(VkDevice, H, P);
    vkCreatePipelineLayout: fn(VkDevice, *const VkPipelineLayoutCreateInfo, P, *mut H) -> VkResult;
    vkDestroyPipelineLayout: fn(VkDevice, H, P);
    vkCreateGraphicsPipelines: fn(VkDevice, H, u32, *const VkGraphicsPipelineCreateInfo, P, *mut H) -> VkResult;
    vkDestroyPipeline: fn(VkDevice, H, P);
    vkCreateDescriptorPool: fn(VkDevice, *const VkDescriptorPoolCreateInfo, P, *mut H) -> VkResult;
    vkDestroyDescriptorPool: fn(VkDevice, H, P);
    vkAllocateDescriptorSets: fn(VkDevice, *const VkDescriptorSetAllocateInfo, *mut H) -> VkResult;
    vkFreeDescriptorSets: fn(VkDevice, H, u32, *const H) -> VkResult;
    vkUpdateDescriptorSets: fn(VkDevice, u32, *const VkWriteDescriptorSet, u32, P);
    vkCreateRenderPass: fn(VkDevice, *const VkRenderPassCreateInfo, P, *mut H) -> VkResult;
    vkDestroyRenderPass: fn(VkDevice, H, P);
    vkCreateFramebuffer: fn(VkDevice, *const VkFramebufferCreateInfo, P, *mut H) -> VkResult;
    vkDestroyFramebuffer: fn(VkDevice, H, P);

    vkCreateCommandPool: fn(VkDevice, *const VkCommandPoolCreateInfo, P, *mut H) -> VkResult;
    vkDestroyCommandPool: fn(VkDevice, H, P);
    vkAllocateCommandBuffers: fn(VkDevice, *const VkCommandBufferAllocateInfo, *mut VkCommandBuffer) -> VkResult;
    vkBeginCommandBuffer: fn(VkCommandBuffer, *const VkCommandBufferBeginInfo) -> VkResult;
    vkEndCommandBuffer: fn(VkCommandBuffer) -> VkResult;
    vkResetCommandBuffer: fn(VkCommandBuffer, VkFlags) -> VkResult;
    vkCreateFence: fn(VkDevice, *const VkSimpleCreateInfo, P, *mut H) -> VkResult;
    vkDestroyFence: fn(VkDevice, H, P);
    vkWaitForFences: fn(VkDevice, u32, *const H, VkBool32, u64) -> VkResult;
    vkResetFences: fn(VkDevice, u32, *const H) -> VkResult;
    vkCreateSemaphore: fn(VkDevice, *const VkSimpleCreateInfo, P, *mut H) -> VkResult;
    vkDestroySemaphore: fn(VkDevice, H, P);
    vkQueueSubmit: fn(VkQueue, u32, *const VkSubmitInfo, H) -> VkResult;

    vkCmdPipelineBarrier: fn(VkCommandBuffer, VkFlags, VkFlags, VkFlags, u32, P, u32, P, u32, *const VkImageMemoryBarrier);
    vkCmdCopyBufferToImage: fn(VkCommandBuffer, H, H, i32, u32, *const VkBufferImageCopy);
    vkCmdCopyImage: fn(VkCommandBuffer, H, i32, H, i32, u32, *const VkImageCopy);
    vkCmdBeginRenderPass: fn(VkCommandBuffer, *const VkRenderPassBeginInfo, i32);
    vkCmdEndRenderPass: fn(VkCommandBuffer);
    vkCmdBindPipeline: fn(VkCommandBuffer, i32, H);
    vkCmdSetViewport: fn(VkCommandBuffer, u32, u32, *const VkViewport);
    vkCmdSetScissor: fn(VkCommandBuffer, u32, u32, *const VkRect2D);
    vkCmdPushConstants: fn(VkCommandBuffer, H, VkFlags, u32, u32, P);
    vkCmdBindVertexBuffers: fn(VkCommandBuffer, u32, u32, *const H, *const VkDeviceSize);
    vkCmdBindDescriptorSets: fn(VkCommandBuffer, i32, H, u32, u32, *const H, u32, *const u32);
    vkCmdDraw: fn(VkCommandBuffer, u32, u32, u32, u32);
}

/// A call's result: `Err("vkCreateInstance: VK_ERROR_INCOMPATIBLE_DRIVER (no Vulkan driver)")`.
pub fn check(r: VkResult, call: &str) -> Result<(), String> {
    let name = match r {
        VK_SUCCESS => return Ok(()),
        -1 => "VK_ERROR_OUT_OF_HOST_MEMORY",
        -2 => "VK_ERROR_OUT_OF_DEVICE_MEMORY",
        -3 => "VK_ERROR_INITIALIZATION_FAILED",
        -4 => "VK_ERROR_DEVICE_LOST",
        -6 => "VK_ERROR_LAYER_NOT_PRESENT",
        -7 => "VK_ERROR_EXTENSION_NOT_PRESENT",
        -9 => "VK_ERROR_INCOMPATIBLE_DRIVER (no Vulkan driver)",
        -1000000000 => "VK_ERROR_SURFACE_LOST_KHR",
        -1000000001 => "VK_ERROR_NATIVE_WINDOW_IN_USE_KHR",
        _ => return Err(format!("{call}: VkResult {r}")),
    };
    Err(format!("{call}: {name}"))
}
