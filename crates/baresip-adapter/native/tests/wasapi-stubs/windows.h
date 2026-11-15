/* Fault injection only: production WASAPI workers, portable COM stand-ins. */
#ifndef SIP_WASAPI_STUB_WINDOWS_H
#define SIP_WASAPI_STUB_WINDOWS_H
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
typedef int32_t HRESULT;
typedef uint32_t DWORD;
typedef uint16_t WORD;
typedef unsigned UINT;
typedef unsigned char BYTE;
typedef wchar_t *LPWSTR;
typedef const wchar_t *LPCWSTR;
typedef struct { int value; } GUID;
static const GUID CLSID_MMDeviceEnumerator = {0}, IID_IMMDeviceEnumerator = {1};
static const GUID IID_IAudioClient = {2}, IID_IAudioCaptureClient = {3};
static const GUID IID_IAudioRenderClient = {4};
#define FAILED(hr) ((hr) < 0)
#define SUCCEEDED(hr) ((hr) >= 0)
#define S_OK 0
#define E_ACCESSDENIED ((HRESULT)-1)
#define E_FAIL ((HRESULT)-2)
#define AUDCLNT_E_DEVICE_INVALIDATED ((HRESULT)-3)
#define AUDCLNT_E_SERVICE_NOT_RUNNING ((HRESULT)-4)
#define AUDCLNT_E_RESOURCES_INVALIDATED ((HRESULT)-5)
#define AUDCLNT_S_BUFFER_EMPTY 1
#define COINIT_MULTITHREADED 0
#define COINIT_DISABLE_OLE1DDE 0
#define COINIT_SPEED_OVER_MEMORY 0
#define CLSCTX_ALL 0
#define CP_UTF8 65001
#define eRender 0
#define eCapture 1
#define eCommunications 2
#define WAVE_FORMAT_PCM 1
#define AUDCLNT_SHAREMODE_SHARED 0
#define AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM 1
#define AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY 2
#define AUDCLNT_BUFFERFLAGS_SILENT 2
typedef struct { WORD wFormatTag, nChannels; DWORD nSamplesPerSec, nAvgBytesPerSec;
    WORD nBlockAlign, wBitsPerSample, cbSize; } WAVEFORMATEX;
typedef struct { int unused; } IMMDeviceEnumerator;
typedef struct { int input, id; } IMMDevice;
typedef struct { int input, id; WAVEFORMATEX format; } IAudioClient;
typedef IAudioClient IAudioCaptureClient;
typedef IAudioClient IAudioRenderClient;
static HRESULT CoInitializeEx(void *arg, unsigned flags);
static void CoUninitialize(void);
static HRESULT CoCreateInstance(const GUID *cls, void *outer, unsigned context,
    const GUID *iid, void **result);
static HRESULT IMMDeviceEnumerator_GetDefaultAudioEndpoint(IMMDeviceEnumerator *enumerator,
    int input, int role, IMMDevice **device);
static HRESULT IMMDeviceEnumerator_GetDevice(IMMDeviceEnumerator *enumerator,
    LPCWSTR id, IMMDevice **device);
static HRESULT IMMDevice_GetId(IMMDevice *device, LPWSTR *id);
static HRESULT IMMDevice_Activate(IMMDevice *device, const GUID *iid,
    unsigned context, void *parameters, void **client);
static HRESULT IAudioClient_GetMixFormat(IAudioClient *client, WAVEFORMATEX **format);
static HRESULT IAudioClient_Initialize(IAudioClient *client, int mode, DWORD flags,
    int64_t duration, int64_t periodicity, const WAVEFORMATEX *format, void *session);
static HRESULT IAudioClient_GetService(IAudioClient *client, const GUID *iid, void **service);
static HRESULT IAudioClient_GetBufferSize(IAudioClient *client, uint32_t *frames);
static HRESULT IAudioClient_Start(IAudioClient *client);
static HRESULT IAudioClient_Stop(IAudioClient *client);
static HRESULT IAudioClient_GetCurrentPadding(IAudioClient *client, uint32_t *frames);
static HRESULT IAudioCaptureClient_GetNextPacketSize(IAudioClient *client, uint32_t *frames);
static HRESULT IAudioCaptureClient_GetBuffer(IAudioClient *client, BYTE **data,
    uint32_t *frames, DWORD *flags, void *position, void *counter);
static HRESULT IAudioCaptureClient_ReleaseBuffer(IAudioClient *client, uint32_t frames);
static HRESULT IAudioRenderClient_GetBuffer(IAudioClient *client, uint32_t frames, BYTE **data);
static HRESULT IAudioRenderClient_ReleaseBuffer(IAudioClient *client, uint32_t frames, DWORD flags);
static int MultiByteToWideChar(unsigned page, DWORD flags, const char *source,
    int length, LPWSTR output, int capacity);
static int WideCharToMultiByte(unsigned page, DWORD flags, LPCWSTR source,
    int length, char *output, int capacity, void *fallback, void *used);
#define CoTaskMemFree free
#define IMMDeviceEnumerator_Release free
#define IMMDevice_Release free
#define IAudioClient_Release free
#define IAudioCaptureClient_Release(client) ((void)(client))
#define IAudioRenderClient_Release(client) ((void)(client))
#endif
