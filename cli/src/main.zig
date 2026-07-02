const std = @import("std");
const builtin = @import("builtin");
const lib = @import("lib.zig");

// Le Coq CLI - Offline intelligence tool
// Static binary, memory-safe, sovereign

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();
    
    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);
    
    if (args.len < 2) {
        try printHelp();
        return;
    }
    
    const command = args[1];
    
    if (std.mem.eql(u8, command, "search")) {
        try cmdSearch(allocator, args[2..]);
    } else if (std.mem.eql(u8, command, "list")) {
        try cmdList(allocator, args[2..]);
    } else if (std.mem.eql(u8, command, "graph")) {
        try cmdGraph(allocator, args[2..]);
    } else if (std.mem.eql(u8, command, "ingest")) {
        try cmdIngest(allocator, args[2..]);
    } else if (std.mem.eql(u8, command, "export")) {
        try cmdExport(allocator, args[2..]);
    } else if (std.mem.eql(u8, command, "help") or std.mem.eql(u8, command, "--help")) {
        try printHelp();
    } else {
        std.debug.print("Unknown command: {s}\n", .{command});
        try printHelp();
        std.process.exit(1);
    }
}

fn printHelp() !void {
    const help = 
        \\Le Coq CLI v0.1.0 - Geopolitical Intelligence Tool
        \\
        \\Usage: le-coq <command> [options]
        \\
        \\Commands:
        \\  search <query>     Search entities and relationships
        \\  list               List all entities
        \\  graph <entity_id>  Visualize entity network (ASCII/JSON)
        \\  ingest <source>    Ingest data from OSINT source
        \\  export <format>    Export analysis (PDF/JSON/CSV)
        \\  help               Show this help message
        \\
        \\Options:
        \\  --offline          Run in offline mode (no network)
        \\  --verbose          Enable verbose output
        \\  --output <file>    Write output to file
        \\
        \\Examples:
        \\  le-coq search "Emmanuel Macron"
        \\  le-coq list
        \\  le-coq graph org_12345 --format json
        \\  le-coq ingest opencorporates --query "TotalEnergies"
        \\  le-coq export pdf --output report.pdf
        \\
    ;
    try std.io.getStdErr().writer().print("{s}", .{help});
}

fn cmdSearch(allocator: std.mem.Allocator, args: []const []const u8) !void {
    _ = allocator; // Suppress unused parameter warning
    if (args.len == 0) {
        std.debug.print("Error: search query required\n", .{});
        std.process.exit(1);
    }
    
    const query = args[0];
    std.debug.print("🔍 Searching for: {s}\n", .{query});
    
    // TODO: Connect to backend API or local cache
    // TODO: Implement full-text search
    // TODO: Return ranked results
    
    std.debug.print("⚠️  Search not yet implemented\n", .{});
}

fn cmdList(allocator: std.mem.Allocator, args: []const []const u8) !void {
    _ = allocator; // Suppress unused parameter warning
    _ = args; // Suppress unused parameter warning
    
    std.debug.print("📋 Listing all entities\n", .{});
    
    // TODO: Connect to backend API or local cache
    // TODO: Return all entities
    
    std.debug.print("⚠️  List not yet implemented\n", .{});
}

fn cmdGraph(allocator: std.mem.Allocator, args: []const []const u8) !void {
    _ = allocator; // Suppress unused parameter warning
    if (args.len == 0) {
        std.debug.print("Error: entity ID required\n", .{});
        std.process.exit(1);
    }
    
    const entity_id = args[0];
    std.debug.print("🕸️  Building graph for: {s}\n", .{entity_id});
    
    // TODO: Query graph database
    // TODO: Render ASCII graph or JSON output
    // TODO: Support --format flag (ascii/json/dot)
    
    std.debug.print("⚠️  Graph visualization not yet implemented\n", .{});
}

fn cmdIngest(allocator: std.mem.Allocator, args: []const []const u8) !void {
    _ = allocator; // Suppress unused parameter warning
    if (args.len == 0) {
        std.debug.print("Error: source name required\n", .{});
        std.process.exit(1);
    }
    
    const source = args[0];
    std.debug.print("📥 Ingesting from: {s}\n", .{source});
    
    // TODO: Call Python plugin via FFI or HTTP
    // TODO: Store results in local database
    // TODO: Show progress bar
    
    std.debug.print("⚠️  Ingestion not yet implemented\n", .{});
}

fn cmdExport(allocator: std.mem.Allocator, args: []const []const u8) !void {
    _ = allocator; // Suppress unused parameter warning
    if (args.len == 0) {
        std.debug.print("Error: export format required (pdf/json/csv)\n", .{});
        std.process.exit(1);
    }
    
    const format = args[0];
    std.debug.print("📄 Exporting as: {s}\n", .{format});
    
    // TODO: Generate report in specified format
    // TODO: Support --output flag for file path
    
    std.debug.print("⚠️  Export not yet implemented\n", .{});
}