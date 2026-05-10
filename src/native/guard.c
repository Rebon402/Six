#include <windows.h>
#include <stdio.h>
#include <stdint.h>

// Hardened Guard: Advanced anti-tamper logic in C
// C is ideal for direct Win32 API manipulation
typedef struct {
    uint32_t checksum;
    uint32_t timestamp;
} RuntimeState;

static RuntimeState global_state = {0, 0};

void six_guard_init() {
    // Check if being run under a debugger via low-level PEB check
    // This is more "hardcore" than IsDebuggerPresent()
    void* peb = NULL;
    #ifdef _WIN64
        peb = (void*)__readgsqword(0x60);
    #else
        peb = (void*)__readfsdword(0x30);
    #endif

    uint8_t being_debugged = *(uint8_t*)((uintptr_t)peb + 2);
    if (being_debugged) {
        fprintf(stderr, "[SixGUARD] HIGH THREAT DETECTED: Hardware Debugger Found.\n");
        ExitProcess(0xDEAD);
    }
}

static HANDLE lock_file_handle = INVALID_HANDLE_VALUE;
static char current_lock_path[MAX_PATH];

void six_guard_lock_init(const char* filename) {
    snprintf(current_lock_path, MAX_PATH, "%s.lock", filename);
    
    lock_file_handle = CreateFileA(
        current_lock_path,
        GENERIC_READ | GENERIC_WRITE,
        0, // NO SHARING = LOCK
        NULL,
        CREATE_ALWAYS,
        FILE_ATTRIBUTE_HIDDEN | FILE_FLAG_DELETE_ON_CLOSE,
        NULL
    );

    if (lock_file_handle == INVALID_HANDLE_VALUE) {
        fprintf(stderr, "[SixGUARD] ERROR: Instance is already locked (%s). Double execution forbidden.\n", current_lock_path);
        ExitProcess(1);
    }
}

void six_guard_lock_release() {
    if (lock_file_handle != INVALID_HANDLE_VALUE) {
        CloseHandle(lock_file_handle);
        DeleteFileA(current_lock_path);
    }
}

int32_t six_guard_heartbeat() {
    // Verify stack integrity or other low-level markers
    // Returning a simple status for the Rust VM to check
    return 0; 
}

// Fallback Native Core in C (if Zig is missing)
void* six_arena_alloc(size_t size) {
    return malloc(size);
}

void six_arena_clear() {
    // Simple free logic or just leave for process exit in fallback mode
}

int32_t six_security_heartbeat() {
    if (IsDebuggerPresent()) return -1;
    return 0;
}

void six_xor_engine(uint8_t* data, size_t len, const uint8_t* keys, size_t key_len) {
    if (key_len == 0) return;
    for (size_t i = 0; i < key_len; i++) {
        uint8_t key = keys[i];
        for (size_t j = 0; j < len; j++) {
            data[j] ^= (key + (uint8_t)(j % 255));
        }
    }
}
