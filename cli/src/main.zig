const std = @import("std");
const builtin = @import("builtin");
const lib = @import("lib.zig");

// Le Coq CLI - Debug & Intelligence Tool
// Static binary, memory-safe, sovereign
//
// MODE DEBUG: Query backend API, inspect plugins, debug graph

// Helper: check if arg matches long or short form
fn isCmd(arg: []const u8, long_name: []const u8, short_name: []const u8) bool {
    // Check exact matches first
    if (std.mem.eql(u8, arg, long_name) or std.mem.eql(u8, arg, short_name)) return true;
    // Check --long form
    if (arg.len > 2 and arg[0] == '-' and arg[1] == '-') {
        if (std.mem.eql(u8, arg[2..], long_name)) return true;
    }
    // Check -s form
    if (arg.len == 2 and arg[0] == '-') {
        if (std.mem.eql(u8, arg[1..], short_name)) return true;
    }
    return false;
}

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
    } else if (isCmd(command, "query", "q")) {
        try queryCommand(stdout, stderr, args[2..]);
    } else if (isCmd(command, "plugins", "p")) {
        try pluginsCommand(stdout, stderr, args[2..]);
    } else if (isCmd(command, "graph", "g")) {
        try graphCommand(stdout, stderr, args[2..]);
    } else if (isCmd(command, "api", "a")) {
        try apiCommand(stdout, stderr, args[2..]);
    } else if (isCmd(command, "health", "h")) {
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
        \\  q[ery] <resource> [OPTIONS]    Query entities, relations, search
        \\  p[lugins] <subcommand>          List, test, validate plugins
        \\  g[raph] <subcommand>            Inspect graph (stats, neighbors)
        \\  a[pi] <method> <endpoint>       Raw API calls (get/post)
        \\  h[ealth]                        Check backend health
        \\  help                            Show this help
        \\
        \\QUERY RESOURCES:
        \\  e[ntities]      --type <type> --q <query> --limit <n>
        \\  r[elations]     --from <id> --to <id> --limit <n>
        \\  s[earch]        --q <query> --limit <n>
        \\  src[ources]     (no options)
        \\
        \\PLUGIN SUBCOMMANDS:
        \\  list | l                      List available plugins
        \\  t[est] <name> --query <q>     Test a plugin
        \\  v[alidate] <name>             Validate plugin config
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
        \\  le-coq q e --type person -l 10        # entities, limit 10
        \\  le-coq q r -l 20                      # relations
        \\  le-coq p list                         # list plugins
        \\  le-coq p t wikipedia -q "France"      # test plugin
        \\  le-coq g stats                        # graph stats
        \\  le-coq a get /api/v1/sources          # API call
        \\  le-coq h                              # health check
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

    if (isCmd(resource, "entities", "e")) {
        try queryEntities(stdout, api_url, api_key, subargs[1..]);
    } else if (isCmd(resource, "relations", "r")) {
        try queryRelations(stdout, api_url, api_key, subargs[1..]);
    } else if (isCmd(resource, "search", "s")) {
        try querySearch(stdout, api_url, api_key, subargs[1..]);
    } else if (isCmd(resource, "sources", "src")) {
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

    var format: []const u8 = "table"; // default
    var limit: []const u8 = "10"; // default

    try query_params.append('?');
    var first = true;

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (isCmd(args[i], "type", "t") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "type={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (isCmd(args[i], "q", "q") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "q={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (isCmd(args[i], "limit", "l") and i + 1 < args.len) {
            limit = args[i + 1];
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "limit={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (isCmd(args[i], "format", "f") and i + 1 < args.len) {
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

    var format: []const u8 = "table"; // default

    try query_params.append('?');
    var first = true;

    var i: usize = 0;
    while (i < args.len) : (i += 1) {
        if (isCmd(args[i], "from", "f") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "from={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (isCmd(args[i], "to", "t") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "to={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (isCmd(args[i], "limit", "l") and i + 1 < args.len) {
            if (!first) try query_params.append('&');
            try std.fmt.format(query_params.writer(), "limit={s}", .{args[i + 1]});
            first = false;
            i += 1;
        } else if (isCmd(args[i], "format", "f") and i + 1 < args.len) {
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

    // Parse JSON array and print each source
    const allocator = std.heap.page_allocator;
    const tmp_path = try std.fmt.allocPrint(allocator, "/tmp/lecoq_sources_{d}.json", .{std.time.timestamp()});
    defer allocator.free(tmp_path);
    
    var file = try std.fs.createFileAbsolute(tmp_path, .{});
    try file.writeAll(response);
    file.close();
    
    const cmd = try std.fmt.allocPrint(allocator, "jq -r '.[]' '{s}' 2>/dev/null || echo '{s}'", .{ tmp_path, response });
    defer allocator.free(cmd);
    
    const result = std.process.Child.exec(.{
        .allocator = allocator,
        .argv = &[_][]const u8{ "sh", "-c", cmd },
    }) catch {
        try stdout.print("{s}\n", .{response});
        return;
    };
    defer allocator.free(result.stderr);
    
    try stdout.print("Active plugins:\n", .{});
    try stdout.print("{s}\n", .{result.stdout});
    
    _ = std.fs.deleteFileAbsolute(tmp_path) catch {};
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
    } else if (isCmd(subcmd, "test", "t")) {
        if (subargs.len < 2) {
            try stderr.print("Usage: le-coq plugins test <source> --query <q>\n", .{});
            std.process.exit(1);
        }
        try pluginsTest(stdout, api_url, api_key, subargs[1..]);
    } else if (isCmd(subcmd, "validate", "v")) {
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
        if (isCmd(args[i], "query", "q") and i + 1 < args.len) {
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
    
    // Build JSON body: {"source": "...", "query": {"q": "..."}}
    const body = try std.fmt.allocPrint(allocator, "{{\"source\":\"{s}\",\"query\":{{\"q\":\"{s}\"}}}}", .{source, query.?});
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
        if (isCmd(subargs[i], "body", "b") and i + 1 < subargs.len) {
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
    const allocator = std.heap.page_allocator;
    
    if (std.mem.eql(u8, format, "json")) {
        // Write to temp file, then jq for pretty print
        const tmp_path = try std.fmt.allocPrint(allocator, "/tmp/lecoq_{s}_{d}.json", .{ resource_type, std.time.timestamp() });
        defer allocator.free(tmp_path);
        
        // Write JSON to file
        var file = try std.fs.createFileAbsolute(tmp_path, .{});
        try file.writeAll(json_response);
        file.close();
        
        // jq the file
        const jq_cmd = try std.fmt.allocPrint(allocator, "jq '.' '{s}' && rm -f '{s}'", .{ tmp_path, tmp_path });
        defer allocator.free(jq_cmd);
        
        const result = std.process.Child.exec(.{
            .allocator = allocator,
            .argv = &[_][]const u8{ "sh", "-c", jq_cmd },
        }) catch {
            try stdout.print("{s}\n", .{json_response});
            return;
        };
        defer allocator.free(result.stderr);
        try stdout.print("{s}", .{result.stdout});
    } else if (std.mem.eql(u8, format, "table")) {
        try formatAsTable(stdout, json_response, resource_type);
    } else if (std.mem.eql(u8, format, "csv")) {
        try formatAsCsv(stdout, json_response, resource_type);
    } else {
        try stdout.print("{s}\n", .{json_response});
    }
}

fn formatAsTable(stdout: anytype, json_response: []const u8, resource_type: []const u8) !void {
    const allocator = std.heap.page_allocator;
    
    // Check if empty array
    if (std.mem.eql(u8, std.mem.trim(u8, json_response, " \n\t"), "[]")) {
        try stdout.print("No {s} found.\n", .{resource_type});
        return;
    }
    
    // Write to temp file
    const tmp_path = try std.fmt.allocPrint(allocator, "/tmp/lecoq_{s}_{d}.json", .{ resource_type, std.time.timestamp() });
    defer allocator.free(tmp_path);
    
    var file = try std.fs.createFileAbsolute(tmp_path, .{});
    try file.writeAll(json_response);
    file.close();
    
    // Extract fields and format as table with jq + column
    const table_cmd = switch (resource_type[0]) {
        'e' => try std.fmt.allocPrint(allocator, "jq -r '.[] | [.id, .name, .type] | @tsv' '{s}' | column -t -s $'\\t' 2>/dev/null || jq -r '.[] | \"\\(.id)\\t\\(.name)\\t\\(.type)\"' '{s}'", .{ tmp_path, tmp_path }),
        'r' => try std.fmt.allocPrint(allocator, "jq -r '.[] | [.from, .to, .type] | @tsv' '{s}' | column -t -s $'\\t' 2>/dev/null || jq -r '.[] | \"\\(.from)\\t\\(.to)\\t\\(.type)\"' '{s}'", .{ tmp_path, tmp_path }),
        's' => try std.fmt.allocPrint(allocator, "jq -r '.[] | \"\\(.)\"' '{s}' | column -t 2>/dev/null || cat '{s}'", .{ tmp_path, tmp_path }),
        else => try std.fmt.allocPrint(allocator, "jq '.' '{s}'", .{tmp_path}),
    };
    defer allocator.free(table_cmd);
    
    const result = std.process.Child.exec(.{
        .allocator = allocator,
        .argv = &[_][]const u8{ "sh", "-c", table_cmd },
    }) catch {
        // Fallback: raw JSON
        try stdout.print("{s}\n", .{json_response});
        return;
    };
    defer allocator.free(result.stderr);
    try stdout.print("{s}\n", .{result.stdout});
    
    // Cleanup
    _ = std.fs.deleteFileAbsolute(tmp_path) catch {};
}

fn formatAsCsv(stdout: anytype, json_response: []const u8, resource_type: []const u8) !void {
    const allocator = std.heap.page_allocator;
    
    // Write to temp file
    const tmp_path = try std.fmt.allocPrint(allocator, "/tmp/lecoq_{s}_{d}.json", .{ resource_type, std.time.timestamp() });
    defer allocator.free(tmp_path);
    
    var file = try std.fs.createFileAbsolute(tmp_path, .{});
    try file.writeAll(json_response);
    file.close();
    
    // Convert to CSV with jq
    const csv_cmd = switch (resource_type[0]) {
        'e' => try std.fmt.allocPrint(allocator, "jq -r '.[] | [.id, .name, .type] | @csv' '{s}'", .{tmp_path}),
        'r' => try std.fmt.allocPrint(allocator, "jq -r '.[] | [.from, .to, .type] | @csv' '{s}'", .{tmp_path}),
        else => try std.fmt.allocPrint(allocator, "jq -r '.[] | @csv' '{s}'", .{tmp_path}),
    };
    defer allocator.free(csv_cmd);
    
    const result = std.process.Child.exec(.{
        .allocator = allocator,
        .argv = &[_][]const u8{ "sh", "-c", csv_cmd },
    }) catch {
        try stdout.print("{s}\n", .{json_response});
        return;
    };
    defer allocator.free(result.stderr);
    try stdout.print("{s}\n", .{result.stdout});
    
    // Cleanup
    _ = std.fs.deleteFileAbsolute(tmp_path) catch {};
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
