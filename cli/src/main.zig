const std = @import("std");
const builtin = @import("builtin");
const lib = @import("lib.zig");

// Le Coq CLI - Debug & Intelligence Tool
// Static binary, memory-safe, sovereign
//
// MODE DEBUG: Query backend API, inspect plugins, debug graph

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    const stderr = std.io.getStdErr().writer();
    const args = try std.process.argsAlloc(std.heap.page_allocator);
    defer std.process.argsFree(std.heap.page_allocator, args);

    if (args.len < 2) {
        try showHelp(stdout);
        return;
    }

    const command = args[1];

    if (std.mem.eql(u8, command, "help") or std.mem.eql(u8, command, "-h") or std.mem.eql(u8, command, "--help")) {
        try showHelp(stdout);
    } else if (std.mem.eql(u8, command, "query")) {
        try queryCommand(stdout, stderr, args[2..]);
    } else if (std.mem.eql(u8, command, "plugins")) {
        try pluginsCommand(stdout, stderr, args[2..]);
    } else if (std.mem.eql(u8, command, "graph")) {
        try graphCommand(stdout, stderr, args[2..]);
    } else if (std.mem.eql(u8, command, "api")) {
        try apiCommand(stdout, stderr, args[2..]);
    } else if (std.mem.eql(u8, command, "health")) {
        try healthCommand(stdout);
    } else {
        try stderr.print("❌ Unknown command: {s}\n\n", .{command});
        try showHelp(stdout);
        std.process.exit(1);
    }
}

fn showHelp(writer: anytype) !void {
    try printChicken(writer);
    
    try writer.print(
        \\Le Coq CLI - Geopolitical Intelligence Tool
        \\Fabriqué en France 🇫🇷
        \\
        \\USAGE:
        \\  le-coq <COMMAND> [OPTIONS]
        \\
        \\COMMANDS:
        \\  query <resource> [OPTIONS]    Query entities, relations, search
        \\  plugins <subcommand>          List, test, validate plugins
        \\  graph <subcommand>            Inspect graph (stats, neighbors)
        \\  api <method> <endpoint>       Raw API calls (get/post)
        \\  health                        Check backend health
        \\  help                          Show this help
        \\
        \\QUERY RESOURCES:
        \\  entities      --type <type> --q <query> --limit <n>
        \\  relations     --from <id> --to <id> --limit <n>
        \\  search        --q <query> --limit <n>
        \\  sources       (no options)
        \\
        \\PLUGIN SUBCOMMANDS:
        \\  list                          List available plugins
        \\  test <name> --query <q>       Test a plugin
        \\  validate <name>               Validate plugin config
        \\
        \\GRAPH SUBCOMMANDS:
        \\  stats                         Graph statistics
        \\  neighbors --id <entity_id>    Get entity neighbors
        \\
        \\API METHODS:
        \\  get <endpoint>                GET request
        \\  post <endpoint> --body <json> POST request
        \\
        \\EXAMPLES:
        \\  le-coq query entities --type person --limit 10
        \\  le-coq query relations --limit 20
        \\  le-coq plugins list
        \\  le-coq plugins test wikipedia --query "France"
        \\  le-coq graph stats
        \\  le-coq api get /api/v1/sources
        \\  le-coq health
        \\
        \\GLOBAL OPTIONS:
        \\  --api-key <key>               API key (or LE_COQ_API_KEY env)
        \\  --api-url <url>               Backend URL (default: http://localhost:8000)
        \\  --format <fmt>                Output: json|table|csv (default: table)
        \\  --verbose, -v                 Verbose output
        \\
        \\Souverain. Transparent. Puissant.
        \\
    , .{});
}

fn printChicken(writer: anytype) !void {
    try writer.print(
        \\
        \\          \
        \\         (o o)
        \\        (  V  )
        \\       ^^  |  ^^
        \\         (___)
        \\        (_____)
        \\       (_______)
        \\      (_________)
        \\
        \\
    , .{});
}

// Query command: entities, relations, search, sources
fn queryCommand(stdout: anytype, stderr: anytype, subargs: [][]const u8) !void {
    if (subargs.len < 1) {
        try stderr.print("Usage: le-coq query <entities|relations|search|sources> [OPTIONS]\n", .{});
        std.process.exit(1);
    }

    const resource = subargs[0];
    const api_key = getApiKey();
    const api_url = getApiUrl();

    if (std.mem.eql(u8, resource, "entities")) {
        try queryEntities(stdout, api_url, api_key, subargs[1..]);
    } else if (std.mem.eql(u8, resource, "relations")) {
        try queryRelations(stdout, api_url, api_key, subargs[1..]);
    } else if (std.mem.eql(u8, resource, "search")) {
        try querySearch(stdout, api_url, api_key, subargs[1..]);
    } else if (std.mem.eql(u8, resource, "sources")) {
        try querySources(stdout, api_url, api_key);
    } else {
        try stderr.print("Unknown resource: {s}\n", .{resource});
        std.process.exit(1);
    }
}

fn queryEntities(stdout: anytype, api_url: []const u8, api_key: []const u8, args: [][]const u8) !void {
    // Parse args: --type, --q, --limit, --format
    var query_params = std.ArrayList(u8).init(std.heap.page_allocator);
    defer query_params.deinit();

    var format: []const u8 = "json";
    var limit: []const u8 = "20";

    try query_params.append('?');
    var first = true;

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--type") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "type={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (std.mem.eql(u8, args[i], "--q") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "q={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (std.mem.eql(u8, args[i], "--limit") and i + 1 < args.len) {
            limit = args[i + 1];
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "limit={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (std.mem.eql(u8, args[i], "--format") and i + 1 < args.len) {
            format = args[i + 1];
            i += 1;
        }
    }

    const endpoint = try std.fmt.allocPrint(std.heap.page_allocator, "{s}/api/v1/entities{s}", .{ api_url, query_params.items });
    defer std.heap.page_allocator.free(endpoint);

    const response = try makeApiRequestRaw(endpoint, api_key, null);
    defer std.heap.page_allocator.free(response);

    try formatOutput(stdout, response, format, "entities");
}

fn queryRelations(stdout: anytype, api_url: []const u8, api_key: []const u8, args: [][]const u8) !void {
    var query_params = std.ArrayList(u8).init(std.heap.page_allocator);
    defer query_params.deinit();

    var format: []const u8 = "json";

    try query_params.append('?');
    var first = true;

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--from") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "from={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (std.mem.eql(u8, args[i], "--to") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "to={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (std.mem.eql(u8, args[i], "--limit") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "limit={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (std.mem.eql(u8, args[i], "--format") and i + 1 < args.len) {
            format = args[i + 1];
            i += 1;
        }
    }

    const endpoint = try std.fmt.allocPrint(std.heap.page_allocator, "{s}/api/v1/relationships{s}", .{ api_url, query_params.items });
    defer std.heap.page_allocator.free(endpoint);

    const response = try makeApiRequestRaw(endpoint, api_key, null);
    defer std.heap.page_allocator.free(response);

    try formatOutput(stdout, response, format, "relations");
}

fn querySearch(stdout: anytype, api_url: []const u8, api_key: []const u8, args: [][]const u8) !void {
    const body = try buildSearchBody(args);
    const endpoint = try std.fmt.allocPrint(std.heap.page_allocator, "{s}/api/v1/search", .{api_url});
    defer std.heap.page_allocator.free(endpoint);

    try makeApiRequest(stdout, "POST", endpoint, api_key, body);
}

fn querySources(stdout: anytype, api_url: []const u8, api_key: []const u8) !void {
    const endpoint = try std.fmt.allocPrint(std.heap.page_allocator, "{s}/api/v1/sources", .{api_url});
    defer std.heap.page_allocator.free(endpoint);

    const response = try makeApiRequestRaw(endpoint, api_key, null);
    defer std.heap.page_allocator.free(response);

    try stdout.print("Active plugins:\n", .{});
    // Simple formatting for sources list
    try stdout.print("{s}\n", .{response});
}

// Plugins command: list, test, validate
fn pluginsCommand(stdout: anytype, stderr: anytype, subargs: [][]const u8) !void {
    if (subargs.len < 1) {
        try stderr.print("Usage: le-coq plugins <list|test|validate> [OPTIONS]\n", .{});
        std.process.exit(1);
    }

    const subcmd = subargs[0];
    const api_key = getApiKey();
    const api_url = getApiUrl();

    if (std.mem.eql(u8, subcmd, "list")) {
        const endpoint = try std.fmt.allocPrint(std.heap.page_allocator, "{s}/api/v1/sources", .{api_url});
        defer std.heap.page_allocator.free(endpoint);
        try makeApiRequest(stdout, "GET", endpoint, api_key, null);
    } else if (std.mem.eql(u8, subcmd, "test")) {
        if (subargs.len < 2) {
            try stderr.print("Usage: le-coq plugins test <source> --query <q>\n", .{});
            std.process.exit(1);
        }
        try pluginsTest(stdout, api_url, api_key, subargs[1..]);
    } else if (std.mem.eql(u8, subcmd, "validate")) {
        try pluginsValidate(stdout, api_url, api_key, subargs[1..]);
    } else {
        try stderr.print("Unknown subcommand: {s}\n", .{subcmd});
        std.process.exit(1);
    }
}

fn pluginsTest(stdout: anytype, api_url: []const u8, api_key: []const u8, args: [][]const u8) !void {
    const allocator = std.heap.page_allocator;
    
    if (args.len < 1) {
        try stdout.print("Usage: le-coq plugins test <source> --query <q>\n", .{});
        return;
    }
    
    const source = args[0];
    
    // Parse --query arg
    var query: ?[]const u8 = null;
    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        if (std.mem.eql(u8, args[i], "--query") and i + 1 < args.len) {
            query = args[i + 1];
            break;
        }
        i += 1;
    }
    
    if (query == null) {
        try stdout.print("Error: --query required\n", .{});
        return;
    }
    
    // Call ingestion endpoint: POST /api/v1/ingest with body
    const endpoint = try std.fmt.allocPrint(allocator, "{s}/api/v1/ingest", .{api_url});
    defer allocator.free(endpoint);
    
    // Build JSON body: {"source": "...", "query": "..."}
    const body = try std.fmt.allocPrint(allocator, "{{\"source\":\"{s}\",\"query\":\"{s}\"}}", .{source, query.?});
    defer allocator.free(body);
    
    try stdout.print("🧪 Testing plugin '{s}' with query: {s}\n\n", .{source, query.?});
    try makeApiRequest(stdout, "POST", endpoint, api_key, body);
}

fn pluginsValidate(stdout: anytype, api_url: []const u8, api_key: []const u8, args: [][]const u8) !void {
    if (args.len < 1) {
        try stdout.print("Usage: le-coq plugins validate <source>\n", .{});
        return;
    }
    
    const source = args[0];
    
    // Check if source is in the active sources list
    const sources_url = try std.fmt.allocPrint(std.heap.page_allocator, "{s}/api/v1/sources", .{api_url});
    defer std.heap.page_allocator.free(sources_url);
    
    const sources_resp = try makeApiRequestRaw(sources_url, api_key, null);
    defer std.heap.page_allocator.free(sources_resp);
    
    // Simple check if source name appears in response
    const found = std.mem.indexOf(u8, sources_resp, source) != null;
    
    if (found) {
        try stdout.print("✅ Plugin '{s}' is active and registered\n", .{source});
        try stdout.print("   Backend endpoint: POST /api/v1/ingest/{s}\n", .{source});
    } else {
        try stdout.print("❌ Plugin '{s}' not found in active sources\n", .{source});
        try stdout.print("   Available: {s}\n", .{sources_resp});
    }
}

// Graph command: stats, neighbors
fn graphCommand(stdout: anytype, stderr: anytype, subargs: [][]const u8) !void {
    if (subargs.len < 1) {
        try stderr.print("Usage: le-coq graph <stats|neighbors> [OPTIONS]\n", .{});
        std.process.exit(1);
    }

    const subcmd = subargs[0];
    const api_key = getApiKey();
    const api_url = getApiUrl();

    if (std.mem.eql(u8, subcmd, "stats")) {
        try graphStats(stdout, api_url, api_key);
    } else if (std.mem.eql(u8, subcmd, "neighbors")) {
        try graphNeighbors(stdout, api_url, api_key, subargs[1..]);
    } else {
        try stderr.print("Unknown subcommand: {s}\n", .{subcmd});
        std.process.exit(1);
    }
}

fn graphStats(stdout: anytype, api_url: []const u8, api_key: []const u8) !void {
    const allocator = std.heap.page_allocator;
    
    // Get entity count - use a high limit to get all
    const entities_url = try std.fmt.allocPrint(allocator, "{s}/api/v1/entities?limit=1000", .{api_url});
    defer allocator.free(entities_url);
    
    const entities_resp = try makeApiRequestRaw(entities_url, api_key, null);
    defer allocator.free(entities_resp);
    
    // Count entities (simple: count "id": occurrences)
    var entity_count: usize = 0;
    var pos: usize = 0;
    while (std.mem.indexOfPos(u8, entities_resp, pos, "\"id\":")) |id_pos| {
        entity_count += 1;
        pos = id_pos + 1;
    }
    
    // Get relationship count
    const relations_url = try std.fmt.allocPrint(allocator, "{s}/api/v1/relationships?limit=1000", .{api_url});
    defer allocator.free(relations_url);
    
    const relations_resp = try makeApiRequestRaw(relations_url, api_key, null);
    defer allocator.free(relations_resp);
    
    var relation_count: usize = 0;
    if (!std.mem.eql(u8, std.mem.trim(u8, relations_resp, " \n\t"), "[]")) {
        pos = 0;
        while (std.mem.indexOfPos(u8, relations_resp, pos, "\"id\":")) |id_pos| {
            relation_count += 1;
            pos = id_pos + 1;
        }
    }
    
    // Get source count
    const sources_url = try std.fmt.allocPrint(allocator, "{s}/api/v1/sources", .{api_url});
    defer allocator.free(sources_url);
    
    const sources_resp = try makeApiRequestRaw(sources_url, api_key, null);
    defer allocator.free(sources_resp);
    
    var source_count: usize = 0;
    pos = 0;
    while (std.mem.indexOfPos(u8, sources_resp, pos, "\"")) |quote_pos| {
        source_count += 1;
        pos = quote_pos + 1;
    }
    source_count = source_count / 2; // Each source has 2 quotes
    
    try stdout.print("\n📊 Graph Statistics\n", .{});
    try stdout.print("{s}\n", .{"─────────────────────"});
    try stdout.print("Entities:      {d}\n", .{entity_count});
    try stdout.print("Relationships: {d}\n", .{relation_count});
    try stdout.print("Sources:       {d}\n", .{source_count});
    try stdout.print("\n", .{});
}

fn graphNeighbors(stdout: anytype, api_url: []const u8, api_key: []const u8, args: [][]const u8) !void {
    _ = api_url;
    _ = api_key;
    _ = args;
    
    // TODO: Implement neighbors query
    // Need backend endpoint: GET /api/v1/entities/{id}/neighbors
    try stdout.print("⚠️  graph neighbors not yet implemented\n", .{});
    try stdout.print("Backend endpoint required: GET /api/v1/entities/<id>/neighbors\n", .{});
}

// API command: get, post
fn apiCommand(stdout: anytype, stderr: anytype, subargs: [][]const u8) !void {
    if (subargs.len < 2) {
        try stderr.print("Usage: le-coq api <get|post> <endpoint> [--body <json>]\n", .{});
        std.process.exit(1);
    }

    const method = subargs[0];
    const endpoint = subargs[1];
    const api_key = getApiKey();
    const api_url = getApiUrl();

    var body: ?[]const u8 = null;
    var i: usize = 2;
    while (i < subargs.len) : (i += 1) {
        if (std.mem.eql(u8, subargs[i], "--body") and i + 1 < subargs.len) {
            body = subargs[i + 1];
            i += 1;
        }
    }

    const full_endpoint = try std.fmt.allocPrint(std.heap.page_allocator, "{s}{s}", .{ api_url, endpoint });
    defer std.heap.page_allocator.free(full_endpoint);

    try makeApiRequest(stdout, method, full_endpoint, api_key, body);
}

// Health command
fn healthCommand(stdout: anytype) !void {
    const api_url = getApiUrl();
    const endpoint = try std.fmt.allocPrint(std.heap.page_allocator, "{s}/health", .{api_url});
    defer std.heap.page_allocator.free(endpoint);

    try makeApiRequest(stdout, "GET", endpoint, "", null);
}

// HTTP client - returns raw response
fn makeApiRequestRaw(url: []const u8, api_key: []const u8, body: ?[]const u8) ![]u8 {
    const allocator = std.heap.page_allocator;

    // Build curl command
    var curl_args = std.ArrayList([]const u8).init(allocator);
    defer curl_args.deinit();

    try curl_args.append("curl");
    try curl_args.append("-s");
    try curl_args.append("-H");
    try curl_args.append(try std.fmt.allocPrint(allocator, "X-API-Key: {s}", .{api_key}));

    if (body) |b| {
        try curl_args.append("-H");
        try curl_args.append("Content-Type: application/json");
        try curl_args.append("-d");
        try curl_args.append(b);
    }

    try curl_args.append(url);

    // Execute curl
    const result = try std.ChildProcess.exec(.{
        .allocator = allocator,
        .argv = curl_args.items,
    });
    defer allocator.free(result.stderr);

    if (result.term != .Exited or result.term.Exited != 0) {
        allocator.free(result.stdout);
        return error.HttpRequestFailed;
    }

    return result.stdout;
}

// HTTP client - prints response
fn makeApiRequest(stdout: anytype, method: []const u8, url: []const u8, api_key: []const u8, body: ?[]const u8) !void {
    _ = method;
    const response = try makeApiRequestRaw(url, api_key, body);
    defer std.heap.page_allocator.free(response);
    try stdout.print("{s}\n", .{response});
}

// Format JSON output as table
fn formatOutput(stdout: anytype, json_response: []const u8, format: []const u8, resource_type: []const u8) !void {
    if (std.mem.eql(u8, format, "json")) {
        try stdout.print("{s}\n", .{json_response});
    } else if (std.mem.eql(u8, format, "table")) {
        try formatAsTable(stdout, json_response, resource_type);
    } else if (std.mem.eql(u8, format, "csv")) {
        try formatAsCsv(stdout, json_response, resource_type);
    } else {
        try stdout.print("{s}\n", .{json_response});
    }
}

fn formatAsTable(stdout: anytype, json_response: []const u8, resource_type: []const u8) !void {
    // Use jq for table formatting if available, else print JSON
    
    // Check if empty array
    if (std.mem.eql(u8, std.mem.trim(u8, json_response, " \n\t"), "[]")) {
        try stdout.print("No {s} found.\n", .{resource_type});
        return;
    }

    // For now, just print JSON - user can pipe to jq
    try stdout.print("{s}\n", .{json_response});
}

fn formatAsCsv(stdout: anytype, json_response: []const u8, resource_type: []const u8) !void {
    _ = resource_type;
    // Simple CSV - just output JSON for now
    try stdout.print("{s}\n", .{json_response});
}

fn buildSearchBody(args: [][]const u8) ![]const u8 {
    _ = args;
    // Build JSON body for search
    return "{\"query\": \"\"}";
}

fn getApiKey() []const u8 {
    return std.process.getEnvVarOwned(std.heap.page_allocator, "LE_COQ_API_KEY") catch "test";
}

fn getApiUrl() []const u8 {
    return std.process.getEnvVarOwned(std.heap.page_allocator, "LE_COQ_API_URL") catch "http://localhost:8000";
}
