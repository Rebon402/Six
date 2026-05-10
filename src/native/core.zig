const std = @import("std");
const windows = std.os.windows;

// Global Arena for .six Safety Frames
var gpa = std.heap.GeneralPurposeAllocator(.{}){};
var arena = std.heap.ArenaAllocator.init(gpa.allocator());

export fn six_arena_alloc(size: usize) ?*anyopaque {
    const allocator = arena.allocator();
    const ptr = allocator.alloc(u8, size) catch return null;
    return ptr.ptr;
}

export fn six_arena_clear() void {
    _ = arena.reset(.retain_capacity);
}

// Anti-Debugger Heartbeat
// Using direct Windows API via Zig
extern "kernel32" fn IsDebuggerPresent() callconv(windows.WINAPI) windows.BOOL;

export fn six_security_heartbeat() i32 {
    if (IsDebuggerPresent() != 0) {
        return -1;
    }
    return 0;
}

// Optimized XOR Engine
export fn six_xor_engine(data: [*]u8, len: usize, keys: [*]const u8, key_len: usize) void {
    if (key_len == 0) return;
    var i: usize = 0;
    while (i < key_len) : (i += 1) {
        const key = keys[i];
        var j: usize = 0;
        while (j < len) : (j += 1) {
            data[j] ^= (key +% @as(u8, @truncate(j % 255)));
        }
    }
}
