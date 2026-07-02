const std = @import("std");
const builtin = @import("builtin");
const lib = @import("lib.zig");

// Le Coq CLI - Offline intelligence tool
// Static binary, memory-safe, sovereign

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    const args = try std.process.argsAlloc(std.heap.page_allocator);
    defer std.process.argsFree(std.heap.page_allocator, args);

    if (args.len < 2) {
        try showHelp(stdout);
        return;
    }

    const command = args[1];

    if (std.mem.eql(u8, command, "help") or std.mem.eql(u8, command, "-h") or std.mem.eql(u8, command, "--help")) {
        try showHelp(stdout);
    } else if (std.mem.eql(u8, command, "search")) {
        if (args.len < 3) {
            try stdout.print("Usage: le-coq search <query>\n", .{});
            return;
        }
        try searchCommand(stdout, args[2]);
    } else if (std.mem.eql(u8, command, "list")) {
        try listCommand(stdout);
    } else if (std.mem.eql(u8, command, "graph")) {
        if (args.len < 3) {
            try stdout.print("Usage: le-coq graph <entity_id>\n", .{});
            return;
        }
        try stdout.print("Graphing entity: {s}\n", .{args[2]});
        try stdout.print("Implementation pending\n", .{});
    } else if (std.mem.eql(u8, command, "ingest")) {
        if (args.len < 3) {
            try stdout.print("Usage: le-coq ingest <source_name>\n", .{});
            return;
        }
        try stdout.print("Ingesting from source: {s}\n", .{args[2]});
        try stdout.print("Implementation pending\n", .{});
    } else if (std.mem.eql(u8, command, "export")) {
        if (args.len < 3) {
            try stdout.print("Usage: le-coq export <format>\n", .{});
            return;
        }
        try stdout.print("Exporting in format: {s}\n", .{args[2]});
        try stdout.print("Implementation pending\n", .{});
    } else {
        try stdout.print("Unknown command: {s}\n", .{command});
        try showHelp(stdout);
    }
}

fn showHelp(writer: anytype) !void {
    // Print the chicken ASCII art
    try printChicken(writer);
    
    try writer.print("Le Coq - French Chicken 🐔\n", .{});
    try writer.print("Offline intelligence tool\n", .{});
    try writer.print("\n", .{});
    try writer.print("USAGE:\n", .{});
    try writer.print("  le-coq <COMMAND>\n", .{});
    try writer.print("\n", .{});
    try writer.print("COMMANDS:\n", .{});
    try writer.print("  search <query>    Search for entities using Wikipedia\n", .{});
    try writer.print("  list              List available plugins\n", .{});
    try writer.print("  graph <entity_id> Visualize entity relationships\n", .{});
    try writer.print("  ingest <source>   Ingest data from a source\n", .{});
    try writer.print("  export <format>   Export data in specified format\n", .{});
    try writer.print("  help              Show this help message\n", .{});
    try writer.print("\n", .{});
    try writer.print("EXAMPLES:\n", .{});
    try writer.print("  le-coq search \"Emmanuel Macron\"\n", .{});
    try writer.print("  le-coq list\n", .{});
    try writer.print("\n", .{});
    try writer.print("Liberté, Égalité, Fraternité - Blue, White, Red\n", .{});
    try writer.print("\n", .{});
}

fn printChicken(writer: anytype) !void {
    try writer.print("\n", .{});
    try writer.print("          \\\n", .{});
    try writer.print("         (o o)\n", .{});
    try writer.print("        (  V  )\n", .{});
    try writer.print("       ^^  |  ^^\n", .{});
    try writer.print("         (___)\n", .{});
    try writer.print("        (_____)\n", .{});
    try writer.print("       (_______)\n", .{});
    try writer.print("      (_________)\n", .{});
    try writer.print("\n", .{});
}

fn searchCommand(writer: anytype, query: []const u8) !void {
    // For now, just print a message that we would call the Python plugin
    try writer.print("Searching for: {s}\n", .{query});
    try writer.print("This would call the Wikipedia plugin to search for this query.\n", .{});
    try writer.print("Implementation connects to Python plugin system.\n", .{});
    try writer.print("\nTo actually search, run:\n", .{});
    try writer.print("  python3 /home/yaya/le-coq/plugin_bridge.py search \"{s}\"\n", .{query});
}

fn listCommand(writer: anytype) !void {
    // For now, just print a message that we would call the Python plugin
    try writer.print("Available plugins:\n", .{});
    try writer.print("  wikipedia: Free encyclopedia data from Wikipedia\n", .{});
    try writer.print("  opencorporates: Company registry data from OpenCorporates (140M+ entities)\n", .{});
    try writer.print("\nTo actually list plugins, run:\n", .{});
    try writer.print("  python3 /home/yaya/le-coq/plugin_bridge.py list\n", .{});
}