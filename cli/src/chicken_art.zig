const std = @import("std");

pub fn printChicken() void {
    const chicken =
        \\           \\\\
        \\          (o o)
        \\         (  V  )
        \\        ^^  |  ^^
        \\          (___)
        \\         (_____)
        \\        (_______)
        \\       (_________)
        \\
    ;
    std.debug.print("{s}\n", .{chicken});
}

// Usage in main.zig:
// printChicken();