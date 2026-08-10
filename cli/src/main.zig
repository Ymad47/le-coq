const std = @import("std");
const lib = @import("lib.zig");

// Le Coq CLI - System Doctor
// Diagnostic & troubleshooting for the Le Coq platform
// Not an end-user query tool — use the frontend for that.

const ALLOC = std.heap.page_allocator;

const Status = enum { ok, warn, fail };

const CheckResult = struct {
    name: []const u8,
    status: Status,
    detail: []const u8,
};

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    const stderr = std.io.getStdErr().writer();
    const args = try std.process.argsAlloc(ALLOC);
    defer std.process.argsFree(ALLOC, args);

    // No args → doctor (default)
    if (args.len < 2) {
        try doctorCommand(stdout, null);
        return;
    }

    const command = args[1];

    if (eql(command, "help") or eql(command, "-h") or eql(command, "--help")) {
        try showHelp(stdout);
    } else if (eql(command, "doctor") or eql(command, "d")) {
        const filter: ?[]const u8 = if (args.len >= 3) args[2] else null;
        try doctorCommand(stdout, filter);
    } else if (eql(command, "health") or eql(command, "h")) {
        try healthCommand(stdout);
    } else if (eql(command, "logs")) {
        const n: []const u8 = if (args.len >= 3) args[2] else "50";
        try logsCommand(stdout, stderr, n);
    } else if (eql(command, "stats") or eql(command, "s")) {
        try statsCommand(stdout);
    } else if (eql(command, "snapshot")) {
        try snapshotCommand(stdout, stderr);
    } else if (eql(command, "diff")) {
        if (args.len < 3) {
            try stderr.print("Usage: le-coq diff <snapshot.snap>\n", .{});
            std.process.exit(1);
        }
        try diffCommand(stdout, stderr, args[2]);
    } else {
        try stderr.print("❌ Unknown command: {s}\n\n", .{command});
        try showHelp(stdout);
        std.process.exit(1);
    }
}

fn eql(a: []const u8, b: []const u8) bool {
    return std.mem.eql(u8, a, b);
}

fn showHelp(writer: anytype) !void {
    try printChicken(writer);
    try writer.print(
        \\Le Coq CLI — System Doctor 🩺
        \\Diagnostic & troubleshooting for Le Coq.
        \\Frontend handles search — CLI handles health.
        \\
        \\USAGE:
        \\  le-coq [COMMAND] [OPTIONS]
        \\  le-coq            (no command) → full doctor
        \\
        \\COMMANDS:
        \\  doctor, d [plugin]        Full diagnostic, or single plugin check
        \\  health, h                 Quick backend ping
        \\  stats, s                  DB counters (entities / relations / sources)
        \\  logs [N]                  Last N journal lines (default 50)
        \\  snapshot                  Save DB state + git rev → .snap file
        \\  diff <snap>               Compare current DB vs snapshot
        \\  help                      This help
        \\
        \\EXAMPLES:
        \\  le-coq                    # doctor full
        \\  le-coq doctor wikipedia   # health-check one plugin
        \\  le-coq logs 200           # last 200 log lines
        \\  le-coq snapshot           # .lecoq-state/20260810-1930.snap
        \\  le-coq diff .lecoq-state/20260810-1930.snap
        \\
        \\ENV:
        \\  LE_COQ_API_URL      Backend URL (default http://localhost:8000)
        \\  LE_COQ_API_KEY      API key (if backend auth enabled)
        \\  LE_COQ_PG_CONN      PostgreSQL conn string (for snapshot)
        \\                      e.g. "postgresql://postgres@localhost:5432/le_coq"
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

// ============================================================================
// DOCTOR
// ============================================================================

fn doctorCommand(stdout: anytype, filter: ?[]const u8) !void {
    try stdout.print("🐓 Le Coq Doctor\n", .{});
    try stdout.print("────────────────────────────────────────────────\n\n", .{});

    var issues: usize = 0;
    var warns: usize = 0;

    // 1) Backend health
    if (filter == null) {
        const r = checkBackendHealth() catch |e| blk: {
            break :blk CheckResult{
                .name = "backend",
                .status = .fail,
                .detail = @errorName(e),
            };
        };
        try printCheck(stdout, r);
        if (r.status == .fail) issues += 1 else if (r.status == .warn) warns += 1;

        // 2) DB stats
        const db = try checkDbStats();
        try printCheck(stdout, db);
        if (db.status == .fail) issues += 1 else if (db.status == .warn) warns += 1;

        try stdout.print("\n", .{});
    }

    // 3) Each plugin (or single one)
    try stdout.print("🔌 Plugins\n", .{});

    const plugins = listPlugins() catch |e| {
        try stdout.print("   ❌ cannot list plugins: {s}\n", .{@errorName(e)});
        try summary(stdout, issues + 1, warns);
        return;
    };
    defer freeSlice(plugins);

    if (plugins.len == 0) {
        try stdout.print("   (none registered)\n", .{});
    }

    for (plugins) |p| {
        if (filter) |f| {
            if (!eql(p.name, f)) continue;
        }
        const r = try checkPlugin(p.name);
        try printCheck(stdout, r);
        if (r.status == .fail) issues += 1 else if (r.status == .warn) warns += 1;
    }

    try stdout.print("\n", .{});
    try summary(stdout, issues, warns);
}

fn summary(stdout: anytype, issues: usize, warns: usize) !void {
    if (issues == 0 and warns == 0) {
        try stdout.print("✅ All systems nominal.\n", .{});
    } else if (issues == 0) {
        try stdout.print("⚠️  {d} warning{s}. System usable, watch out.\n", .{ warns, plural(warns) });
    } else {
        try stdout.print("❌ {d} failure{s}, {d} warning{s}.\n", .{ issues, plural(issues), warns, plural(warns) });
    }
}

fn plural(n: usize) []const u8 {
    return if (n > 1) "s" else "";
}

fn printCheck(stdout: anytype, r: CheckResult) !void {
    const icon: []const u8 = switch (r.status) {
        .ok => "   ✅",
        .warn => "   ⚠️ ",
        .fail => "   ❌",
    };
    try stdout.print("{s} {s:<14} {s}\n", .{ icon, r.name, r.detail });
}

// ============================================================================
// CHECKS
// ============================================================================

const PluginInfo = struct { name: []const u8 };

fn listPlugins() ![]PluginInfo {
    const url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/sources", .{apiUrl()});
    defer ALLOC.free(url);

    const resp = try rawGet(url);
    defer ALLOC.free(resp);

    // Parse JSON array of strings with std.json
    const parsed = try std.json.parseFromSlice(std.json.Value, ALLOC, resp, .{});
    defer parsed.deinit();

    var list = std.ArrayList(PluginInfo).init(ALLOC);
    if (parsed.value != .array) return list.toOwnedSlice();

    for (parsed.value.array.items) |item| {
        if (item == .string) {
            const dup = try ALLOC.dupe(u8, item.string);
            try list.append(.{ .name = dup });
        }
    }
    return list.toOwnedSlice();
}

fn freeSlice(slice: []PluginInfo) void {
    for (slice) |p| ALLOC.free(p.name);
    ALLOC.free(slice);
}

fn checkBackendHealth() !CheckResult {
    const url = try std.fmt.allocPrint(ALLOC, "{s}/health", .{apiUrl()});
    defer ALLOC.free(url);

    const start = std.time.milliTimestamp();
    const resp = rawGet(url) catch {
        return CheckResult{ .name = "backend", .status = .fail, .detail = "unreachable" };
    };
    defer ALLOC.free(resp);
    const elapsed = std.time.milliTimestamp() - start;

    if (resp.len == 0) {
        return CheckResult{ .name = "backend", .status = .fail, .detail = "empty response" };
    }

    const detail = try std.fmt.allocPrint(ALLOC, "up in {d}ms", .{elapsed});
    return CheckResult{ .name = "backend", .status = .ok, .detail = detail };
}

fn checkDbStats() !CheckResult {
    const url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/entities?limit=1", .{apiUrl()});
    defer ALLOC.free(url);

    const resp = rawGet(url) catch {
        return CheckResult{ .name = "database", .status = .fail, .detail = "query failed" };
    };
    defer ALLOC.free(resp);

    // Quick total via ?limit=1000 full scan would be heavy; just detect "ok"
    if (resp.len > 0) {
        return CheckResult{ .name = "database", .status = .ok, .detail = "reachable" };
    }
    return CheckResult{ .name = "database", .status = .warn, .detail = "empty" };
}

fn checkPlugin(name: []const u8) !CheckResult {
    // Trigger a minimal real fetch to verify the plugin works end-to-end
    const url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/ingest", .{apiUrl()});
    defer ALLOC.free(url);

    const body = try std.fmt.allocPrint(ALLOC,
        \\{{"source":"{s}","query":{{"query":"France"}},"rate_limit":1}}
    , .{name});
    defer ALLOC.free(body);

    const start = std.time.milliTimestamp();
    const resp = rawPost(url, body) catch {
        const dup = try ALLOC.dupe(u8, "call failed");
        return CheckResult{ .name = name, .status = .fail, .detail = dup };
    };
    defer ALLOC.free(resp);
    const elapsed = std.time.milliTimestamp() - start;

    // Detect failure messages in response
    if (std.mem.indexOf(u8, resp, "\"success\":false") != null or
        std.mem.indexOf(u8, resp, "\"success\": false") != null)
    {
        // Try to extract error message
        const msg = extractJsonString(resp, "error") orelse "plugin error";
        const dup = try ALLOC.dupe(u8, msg);
        return CheckResult{ .name = name, .status = .fail, .detail = dup };
    }

    if (std.mem.indexOf(u8, resp, "rate") != null and std.mem.indexOf(u8, resp, "limit") != null) {
        return CheckResult{ .name = name, .status = .warn, .detail = "rate-limited" };
    }

    const status: Status = if (elapsed > 5000) .warn else .ok;
    const detail = try std.fmt.allocPrint(ALLOC, "{d}ms", .{elapsed});
    return CheckResult{ .name = name, .status = status, .detail = detail };
}

fn extractJsonString(haystack: []const u8, key: []const u8) ?[]const u8 {
    const pat = std.fmt.allocPrint(ALLOC, "\"{s}\":\"", .{key}) catch return null;
    defer ALLOC.free(pat);
    const start = std.mem.indexOf(u8, haystack, pat) orelse return null;
    const val_start = start + pat.len;
    const end = std.mem.indexOfPos(u8, haystack, val_start, "\"") orelse return null;
    return haystack[val_start..end];
}

// ============================================================================
// HEALTH / STATS / LOGS / SNAPSHOT / DIFF
// ============================================================================

fn healthCommand(stdout: anytype) !void {
    const url = try std.fmt.allocPrint(ALLOC, "{s}/health", .{apiUrl()});
    defer ALLOC.free(url);
    const resp = rawGet(url) catch {
        try stdout.print("❌ Backend unreachable\n", .{});
        return;
    };
    defer ALLOC.free(resp);
    try stdout.print("🏥 {s}\n", .{resp});
}

fn statsCommand(stdout: anytype) !void {
    try stdout.print("\n📊 Statistics\n─────────────────────\n", .{});

    const e_url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/entities?limit=1000", .{apiUrl()});
    defer ALLOC.free(e_url);
    const e_resp = try rawGet(e_url);
    defer ALLOC.free(e_resp);
    const entities = countOccurrences(e_resp, "\"id\":");

    const r_url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/relationships?limit=1000", .{apiUrl()});
    defer ALLOC.free(r_url);
    const r_resp = try rawGet(r_url);
    defer ALLOC.free(r_resp);
    const relations = countOccurrences(r_resp, "\"id\":");

    const s_url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/sources", .{apiUrl()});
    defer ALLOC.free(s_url);
    const s_resp = try rawGet(s_url);
    defer ALLOC.free(s_resp);
    const quotes = countOccurrences(s_resp, "\"");
    const sources = quotes / 2;

    try stdout.print("Entities:      {d}\n", .{entities});
    try stdout.print("Relationships: {d}\n", .{relations});
    try stdout.print("Sources:       {d}\n\n", .{sources});
}

fn countOccurrences(haystack: []const u8, needle: []const u8) usize {
    var count: usize = 0;
    var pos: usize = 0;
    while (std.mem.indexOfPos(u8, haystack, pos, needle)) |p| {
        count += 1;
        pos = p + 1;
    }
    return count;
}

fn logsCommand(stdout: anytype, stderr: anytype, n: []const u8) !void {
    _ = stderr;
    const cmd = try std.fmt.allocPrint(ALLOC,
        \\journalctl -u le-coq-backend -n {s} --no-pager 2>/dev/null || tail -n {s} /var/log/le-coq.log 2>/dev/null || echo "journalctl unavailable — no logs found"
    , .{ n, n });
    defer ALLOC.free(cmd);

    const result = try std.process.Child.exec(.{
        .allocator = ALLOC,
        .argv = &[_][]const u8{ "sh", "-c", cmd },
    });
    defer ALLOC.free(result.stderr);
    defer ALLOC.free(result.stdout);

    try stdout.print("{s}", .{result.stdout});
}

fn snapshotCommand(stdout: anytype, stderr: anytype) !void {
    // Create .lecoq-state dir if missing
    const dir_path = ".lecoq-state";
    std.fs.cwd().makePath(dir_path) catch {};

    // Timestamp
    const ts = std.time.timestamp();
    const epoch_secs: u64 = @intCast(ts);
    const epoch_day = epoch_secs / 86400;
    const secs_in_day = epoch_secs % 86400;

    // Simple filename (no calendar conversion to keep zig small)
    const filename = try std.fmt.allocPrint(ALLOC, "{s}/{d}-{d}.snap", .{ dir_path, epoch_day, secs_in_day });
    defer ALLOC.free(filename);

    // Get entity + relation counts
    const e_url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/entities?limit=10000", .{apiUrl()});
    defer ALLOC.free(e_url);
    const e_resp = rawGet(e_url) catch {
        try stderr.print("❌ Cannot fetch entities\n", .{});
        return;
    };
    defer ALLOC.free(e_resp);

    const r_url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/relationships?limit=10000", .{apiUrl()});
    defer ALLOC.free(r_url);
    const r_resp = rawGet(r_url) catch {
        try stderr.print("❌ Cannot fetch relationships\n", .{});
        return;
    };
    defer ALLOC.free(r_resp);

    // Count IDs per entity (light: collect distinct "id" values)
    const entity_count = countOccurrences(e_resp, "\"id\":");
    const relation_count = countOccurrences(r_resp, "\"id\":");

    // Git rev
    const git_result = std.process.Child.exec(.{
        .allocator = ALLOC,
        .argv = &[_][]const u8{ "sh", "-c", "cd /home/yaya/le-coq && git rev-parse --short HEAD 2>/dev/null || echo unknown" },
    }) catch null;
    const git_rev: []const u8 = if (git_result) |res| blk: {
        const trimmed = std.mem.trim(u8, res.stdout, " \n\t");
        break :blk trimmed;
    } else "unknown";
    defer if (git_result) |res| {
        ALLOC.free(res.stdout);
        ALLOC.free(res.stderr);
    };

    var file = try std.fs.cwd().createFile(filename, .{});
    defer file.close();

    const content = try std.fmt.allocPrint(ALLOC,
        \\timestamp={d}
        \\git_rev={s}
        \\entities={d}
        \\relations={d}
        \\
    , .{ ts, git_rev, entity_count, relation_count });
    defer ALLOC.free(content);

    try file.writeAll(content);
    try file.writeAll(e_resp);
    try file.writeAll("\n---RELATIONS---\n");
    try file.writeAll(r_resp);

    try stdout.print("📸 Snapshot saved: {s}\n", .{filename});
    try stdout.print("   entities={d} relations={d} git={s}\n", .{ entity_count, relation_count, git_rev });
}

fn diffCommand(stdout: anytype, stderr: anytype, snap_path: []const u8) !void {
    const file = std.fs.cwd().openFile(snap_path, .{}) catch {
        try stderr.print("❌ Cannot open {s}\n", .{snap_path});
        return;
    };
    defer file.close();

    const content = try file.readToEndAlloc(ALLOC, 50 * 1024 * 1024);
    defer ALLOC.free(content);

    // Parse header
    var old_entities: usize = 0;
    var old_relations: usize = 0;
    var old_git: []const u8 = "?";

    var lines = std.mem.splitScalar(u8, content, '\n');
    while (lines.next()) |line| {
        if (std.mem.startsWith(u8, line, "entities=")) {
            old_entities = std.fmt.parseInt(usize, line["entities=".len..], 10) catch 0;
        } else if (std.mem.startsWith(u8, line, "relations=")) {
            old_relations = std.fmt.parseInt(usize, line["relations=".len..], 10) catch 0;
        } else if (std.mem.startsWith(u8, line, "git_rev=")) {
            old_git = line["git_rev=".len..];
        }
    }

    // Current
    const e_url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/entities?limit=10000", .{apiUrl()});
    defer ALLOC.free(e_url);
    const e_resp = try rawGet(e_url);
    defer ALLOC.free(e_resp);
    const cur_entities = countOccurrences(e_resp, "\"id\":");

    const r_url = try std.fmt.allocPrint(ALLOC, "{s}/api/v1/relationships?limit=10000", .{apiUrl()});
    defer ALLOC.free(r_url);
    const r_resp = try rawGet(r_url);
    defer ALLOC.free(r_resp);
    const cur_relations = countOccurrences(r_resp, "\"id\":");

    try stdout.print("📊 Diff vs {s}\n", .{snap_path});
    try stdout.print("─────────────────────\n", .{});
    try stdout.print("   git rev (snap): {s}\n", .{old_git});
    try stdout.print("   entities:    {d:>6} → {d:<6}", .{ old_entities, cur_entities });
    if (cur_entities > old_entities) {
        try stdout.print("  🆕 +{d}\n", .{cur_entities - old_entities});
    } else if (cur_entities < old_entities) {
        try stdout.print("  ➖ -{d}\n", .{old_entities - cur_entities});
    } else {
        try stdout.print("\n", .{});
    }
    try stdout.print("   relations:   {d:>6} → {d:<6}", .{ old_relations, cur_relations });
    if (cur_relations > old_relations) {
        try stdout.print("  🆕 +{d}\n", .{cur_relations - old_relations});
    } else if (cur_relations < old_relations) {
        try stdout.print("  ➖ -{d}\n", .{old_relations - cur_relations});
    } else {
        try stdout.print("\n", .{});
    }
}

// ============================================================================
// HTTP
// ============================================================================

fn rawGet(url: []const u8) ![]u8 {
    return rawRequest("GET", url, null);
}

fn rawPost(url: []const u8, body: []const u8) ![]u8 {
    return rawRequest("POST", url, body);
}

fn rawRequest(method: []const u8, url: []const u8, body: ?[]const u8) ![]u8 {
    var argv = std.ArrayList([]const u8).init(ALLOC);
    defer argv.deinit();

    try argv.append("curl");
    try argv.append("-sS");
    try argv.append("--max-time");
    try argv.append("15");
    try argv.append("-X");
    try argv.append(method);

    const key = apiKey();
    if (key.len > 0) {
        const hdr = try std.fmt.allocPrint(ALLOC, "X-API-Key: {s}", .{key});
        // don't free hdr — used in argv before exec completes leak-safe enough for short-lived CLI
        try argv.append("-H");
        try argv.append(hdr);
    }
    if (body) |b| {
        try argv.append("-H");
        try argv.append("Content-Type: application/json");
        try argv.append("-d");
        try argv.append(b);
    }
    try argv.append(url);

    const result = try std.process.Child.exec(.{
        .allocator = ALLOC,
        .argv = argv.items,
        .max_output_bytes = 100 * 1024 * 1024,
    });
    defer ALLOC.free(result.stderr);

    if (result.term != .Exited or result.term.Exited != 0) {
        ALLOC.free(result.stdout);
        return error.HttpFailed;
    }
    return result.stdout;
}

fn apiUrl() []const u8 {
    return std.process.getEnvVarOwned(ALLOC, "LE_COQ_API_URL") catch "http://localhost:8000";
}

fn apiKey() []const u8 {
    return std.process.getEnvVarOwned(ALLOC, "LE_COQ_API_KEY") catch "";
}
