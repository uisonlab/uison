// COM method indices from the SDK headers (Task6.md, experiment 6).
// A fake object: slot N of its table records N. Calling a method through
// the real interface from the header shows which slot the compiler uses.
// Build (Developer PowerShell / after vcvars64.bat):
//   cl /nologo /EHsc /std:c++17 tools\vtable.cpp /Fe:target\vtable.exe /Fo:target\ && target\vtable.exe
// Output: lines "Interface::Method N"; tools/win.ps1 -CheckVtables compares them with the indices in uison-windows.

#include <d2d1.h>
#include <dwrite.h>
#include <wincodec.h>
#include <d2d1_1.h>
#include <dxgi1_2.h>
#include <d3d11.h>
#include <shobjidl.h>
#include <array>
#include <cstdio>
#include <utility>

static int g_index = -1;
static char g_buf[256];

template <int N>
void* __stdcall stub() {
    g_index = N;
    // For methods returning a struct through a hidden pointer, the caller reads from rax.
    return g_buf;
}

template <int... N>
constexpr auto make_table(std::integer_sequence<int, N...>) {
    return std::array<void*, sizeof...(N)>{(void*)&stub<N>...};
}
static auto g_table = make_table(std::make_integer_sequence<int, 128>{});
static struct { void** vt; } g_fake = {g_table.data()};

template <class I>
I* fake() { return reinterpret_cast<I*>(&g_fake); }

#define SHOW(I, M, CALL)                \
    do {                                \
        g_index = -1;                   \
        fake<I>()->CALL;                \
        printf(#I "::" #M " %d\n", g_index); \
    } while (0)

int main() {
    void* p = nullptr;
    D2D1_RECT_F r{};
    D2D1_ROUNDED_RECT rr{};
    D2D1_ELLIPSE e{};
    D2D1_COLOR_F c{};
    D2D1_POINT_2F pt{};
    D2D1_SIZE_U su{};
    FLOAT f = 0;
    UINT32 u = 0;
    BOOL b = 0;

    SHOW(IUnknown, QueryInterface, QueryInterface(IID_IUnknown, &p));
    SHOW(IUnknown, AddRef, AddRef());
    SHOW(IUnknown, Release, Release());

    SHOW(ID2D1Factory, CreatePathGeometry, CreatePathGeometry((ID2D1PathGeometry**)&p));
    SHOW(ID2D1Factory, CreateStrokeStyle, CreateStrokeStyle(D2D1_STROKE_STYLE_PROPERTIES{}, nullptr, 0, (ID2D1StrokeStyle**)&p));
    SHOW(ID2D1Factory, CreateWicBitmapRenderTarget, CreateWicBitmapRenderTarget(nullptr, D2D1_RENDER_TARGET_PROPERTIES{}, (ID2D1RenderTarget**)&p));
    SHOW(ID2D1Factory, CreateHwndRenderTarget, CreateHwndRenderTarget(D2D1_RENDER_TARGET_PROPERTIES{}, D2D1_HWND_RENDER_TARGET_PROPERTIES{}, (ID2D1HwndRenderTarget**)&p));

    SHOW(ID2D1RenderTarget, CreateSolidColorBrush, CreateSolidColorBrush(&c, nullptr, (ID2D1SolidColorBrush**)&p));
    SHOW(ID2D1RenderTarget, DrawLine, DrawLine(pt, pt, nullptr, 1, nullptr));
    SHOW(ID2D1RenderTarget, DrawRectangle, DrawRectangle(&r, nullptr, 1, nullptr));
    SHOW(ID2D1RenderTarget, FillRectangle, FillRectangle(&r, nullptr));
    SHOW(ID2D1RenderTarget, DrawRoundedRectangle, DrawRoundedRectangle(&rr, nullptr, 1, nullptr));
    SHOW(ID2D1RenderTarget, FillRoundedRectangle, FillRoundedRectangle(&rr, nullptr));
    SHOW(ID2D1RenderTarget, DrawEllipse, DrawEllipse(&e, nullptr, 1, nullptr));
    SHOW(ID2D1RenderTarget, FillEllipse, FillEllipse(&e, nullptr));
    SHOW(ID2D1RenderTarget, DrawGeometry, DrawGeometry(nullptr, nullptr, 1, nullptr));
    SHOW(ID2D1RenderTarget, FillGeometry, FillGeometry(nullptr, nullptr, nullptr));
    SHOW(ID2D1RenderTarget, DrawTextLayout, DrawTextLayout(pt, nullptr, nullptr, D2D1_DRAW_TEXT_OPTIONS_NONE));
    SHOW(ID2D1RenderTarget, SetTextAntialiasMode, SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_DEFAULT));
    SHOW(ID2D1RenderTarget, PushAxisAlignedClip, PushAxisAlignedClip(&r, D2D1_ANTIALIAS_MODE_ALIASED));
    SHOW(ID2D1RenderTarget, PopAxisAlignedClip, PopAxisAlignedClip());
    SHOW(ID2D1RenderTarget, Clear, Clear(&c));
    SHOW(ID2D1RenderTarget, BeginDraw, BeginDraw());
    SHOW(ID2D1RenderTarget, EndDraw, EndDraw(nullptr, nullptr));
    SHOW(ID2D1RenderTarget, SetDpi, SetDpi(96, 96));
    SHOW(ID2D1RenderTarget, GetDpi, GetDpi(&f, &f));
    SHOW(ID2D1RenderTarget, GetSize, GetSize());
    SHOW(ID2D1RenderTarget, GetPixelSize, GetPixelSize());
    SHOW(ID2D1HwndRenderTarget, Resize, Resize(&su));
    SHOW(ID2D1SolidColorBrush, SetColor, SetColor(&c));
    SHOW(ID2D1PathGeometry, Open, Open((ID2D1GeometrySink**)&p));
    SHOW(ID2D1GeometrySink, SetFillMode, SetFillMode(D2D1_FILL_MODE_WINDING));
    SHOW(ID2D1GeometrySink, BeginFigure, BeginFigure(pt, D2D1_FIGURE_BEGIN_FILLED));
    SHOW(ID2D1GeometrySink, AddLines, AddLines(&pt, 1));
    SHOW(ID2D1GeometrySink, EndFigure, EndFigure(D2D1_FIGURE_END_CLOSED));
    SHOW(ID2D1GeometrySink, Close, Close());

    SHOW(IDWriteFactory, GetSystemFontCollection, GetSystemFontCollection((IDWriteFontCollection**)&p, FALSE));
    SHOW(IDWriteFactory, CreateTextFormat, CreateTextFormat(L"", nullptr, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, 1, L"", (IDWriteTextFormat**)&p));
    SHOW(IDWriteFactory, CreateTextLayout, CreateTextLayout(L"", 0, nullptr, 1, 1, (IDWriteTextLayout**)&p));
    SHOW(IDWriteFontCollection, FindFamilyName, FindFamilyName(L"", &u, &b));
    SHOW(IDWriteTextFormat, SetWordWrapping, SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP));
    SHOW(IDWriteTextLayout, GetLineMetrics, GetLineMetrics(nullptr, 0, &u));
    SHOW(IDWriteTextLayout, GetMetrics, GetMetrics(nullptr));

    SHOW(IWICImagingFactory, CreateEncoder, CreateEncoder(GUID_ContainerFormatPng, nullptr, (IWICBitmapEncoder**)&p));
    SHOW(IWICImagingFactory, CreateStream, CreateStream((IWICStream**)&p));
    SHOW(IWICImagingFactory, CreateBitmap, CreateBitmap(1, 1, GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad, (IWICBitmap**)&p));
    SHOW(IWICStream, InitializeFromFilename, InitializeFromFilename(L"", 0));
    SHOW(IWICBitmapSource, GetSize, GetSize(&u, &u));
    SHOW(IWICBitmapEncoder, Initialize, Initialize(nullptr, WICBitmapEncoderNoCache));
    SHOW(IWICBitmapEncoder, CreateNewFrame, CreateNewFrame((IWICBitmapFrameEncode**)&p, nullptr));
    SHOW(IWICBitmapEncoder, Commit, Commit());
    SHOW(IWICBitmapFrameEncode, Initialize, Initialize(nullptr));
    SHOW(IWICBitmapFrameEncode, SetSize, SetSize(1, 1));
    SHOW(IWICBitmapFrameEncode, SetPixelFormat, SetPixelFormat(nullptr));
    SHOW(IWICBitmapFrameEncode, WritePixels, WritePixels(0, 0, 0, nullptr));
    SHOW(IWICBitmapFrameEncode, WriteSource, WriteSource(nullptr, nullptr));
    SHOW(IWICBitmapFrameEncode, Commit, Commit());
    SHOW(IDXGIFactory, EnumAdapters, EnumAdapters(0, (IDXGIAdapter**)&p));
    SHOW(IDXGIAdapter, EnumOutputs, EnumOutputs(0, (IDXGIOutput**)&p));
    SHOW(IDXGIOutput, WaitForVBlank, WaitForVBlank());
    SHOW(IDXGIOutput, GetDesc, GetDesc(nullptr));
    SHOW(IDXGIObject, GetParent, GetParent(__uuidof(IDXGIFactory2), &p));
    SHOW(IDXGIDevice, GetAdapter, GetAdapter((IDXGIAdapter**)&p));
    SHOW(IDXGIFactory2, CreateSwapChainForHwnd, CreateSwapChainForHwnd(nullptr, nullptr, nullptr, nullptr, nullptr, (IDXGISwapChain1**)&p));
    SHOW(IDXGISwapChain, GetBuffer, GetBuffer(0, __uuidof(IDXGISurface), &p));
    SHOW(IDXGISwapChain, ResizeBuffers, ResizeBuffers(0, 0, 0, DXGI_FORMAT_UNKNOWN, 0));
    SHOW(IDXGISwapChain1, Present1, Present1(1, 0, nullptr));
    SHOW(ID2D1Factory1, CreateDevice, CreateDevice(nullptr, (ID2D1Device**)&p));
    SHOW(ID2D1Device, CreateDeviceContext, CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE, (ID2D1DeviceContext**)&p));
    SHOW(ID2D1DeviceContext, CreateBitmapFromDxgiSurface, CreateBitmapFromDxgiSurface(nullptr, (D2D1_BITMAP_PROPERTIES1*)nullptr, (ID2D1Bitmap1**)&p));
    SHOW(ID2D1DeviceContext, SetTarget, SetTarget(nullptr));
    // Images (Task 7.5).
    D2D1_MATRIX_3X2_F m{};
    SHOW(ID2D1RenderTarget, CreateBitmapFromWicBitmap, CreateBitmapFromWicBitmap((IWICBitmapSource*)nullptr, (const D2D1_BITMAP_PROPERTIES*)nullptr, (ID2D1Bitmap**)&p));
    SHOW(ID2D1RenderTarget, CreateBitmapBrush, CreateBitmapBrush((ID2D1Bitmap*)nullptr, (const D2D1_BITMAP_BRUSH_PROPERTIES*)nullptr, (const D2D1_BRUSH_PROPERTIES*)nullptr, (ID2D1BitmapBrush**)&p));
    SHOW(ID2D1RenderTarget, DrawBitmap, DrawBitmap(nullptr, &r, 1.0f, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, &r));
    SHOW(ID2D1Brush, SetTransform, SetTransform(&m));
    SHOW(IWICImagingFactory, CreateDecoderFromStream, CreateDecoderFromStream(nullptr, nullptr, WICDecodeMetadataCacheOnDemand, (IWICBitmapDecoder**)&p));
    SHOW(IWICImagingFactory, CreateFormatConverter, CreateFormatConverter((IWICFormatConverter**)&p));
    SHOW(IWICImagingFactory, CreateBitmapScaler, CreateBitmapScaler((IWICBitmapScaler**)&p));
    SHOW(IWICStream, InitializeFromMemory, InitializeFromMemory((BYTE*)nullptr, 0));
    SHOW(IWICBitmapDecoder, GetFrame, GetFrame(0, (IWICBitmapFrameDecode**)&p));
    SHOW(IWICBitmapScaler, Initialize, Initialize(nullptr, 1, 1, WICBitmapInterpolationModeFant));
    SHOW(IWICFormatConverter, Initialize, Initialize(nullptr, GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, nullptr, 0.0, WICBitmapPaletteTypeCustom));
    SHOW(IWICBitmapSource, CopyPixels, CopyPixels(nullptr, 0, 0, nullptr));
    // examples/disk-usage (folder dialog):
    SHOW(IFileOpenDialog, Show, Show(nullptr));
    SHOW(IFileOpenDialog, SetOptions, SetOptions(0));
    SHOW(IFileOpenDialog, GetOptions, GetOptions((FILEOPENDIALOGOPTIONS*)nullptr));
    SHOW(IFileOpenDialog, SetTitle, SetTitle(nullptr));
    SHOW(IFileOpenDialog, GetResult, GetResult((IShellItem**)&p));
    SHOW(IShellItem, GetDisplayName, GetDisplayName(SIGDN_FILESYSPATH, (LPWSTR*)&p));
    return 0;
}
