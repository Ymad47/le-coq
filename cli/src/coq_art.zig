// ASCII Art for Le Coq CLI
// French Flag Colors: Blue, White, Red

const std = @import("std");

pub fn printCoq() void {
    const coq =
        \\           .--.
        \\          |o_o |
        \\          |:_/ |
        \\         //   \ \
        \\        (|     | )
        \\       /'\_   _/`
        \\       \___)=(___/
        \\
        \\    Le Coq - French Rooster
        \\    Colors: Blue | White | Red
    ;
    std.debug.print("{s}\n", .{coq});
}

// Usage in main.zig:
// printCoq();