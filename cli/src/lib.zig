const std = @import("std");
const mem = std.mem;
const fs = std.fs;

pub const Config = struct {
    backend_url: []const u8,
    offline_mode: bool,
    cache_dir: []const u8,

    pub fn load() !Config {
        return Config{
            .backend_url = "http://localhost:8000",
            .offline_mode = false,
            .cache_dir = "/tmp/le-coq-cache",
        };
    }
};

pub fn fetchEntities(allocator: std.mem.Allocator, backend_url: []const u8) ![]u8 {
    const url = try std.mem.concat(allocator, u8, &[_][]const u8{ backend_url, "/api/v1/entities" });
    defer allocator.free(url);

    // Use curl to fetch entities
    const result = try std.ChildProcess.exec(.{
        .allocator = allocator,
        .argv = &[_][]const u8{ "curl", "-s", url },
    });
    defer {
        allocator.free(result.stderr);
    }

    if (result.term != .Exited or result.term.Exited != 0) {
        allocator.free(result.stdout);
        return error.HttpRequestFailed;
    }

    return result.stdout;
}

pub fn searchEntities(allocator: std.mem.Allocator, backend_url: []const u8, query: []const u8) ![]u8 {
    const url = try std.mem.concat(allocator, u8, &[_][]const u8{ backend_url, "/api/v1/search" });
    defer allocator.free(url);

    // Create JSON payload
    const payload = try std.fmt.allocPrint(allocator, "{{\"query\":\"{s}\"}}", .{query});
    defer allocator.free(payload);

    // Use curl to make the POST request
    const result = try std.ChildProcess.exec(.{
        .allocator = allocator,
        .argv = &[_][]const u8{ "curl", "-s", "-X", "POST", "-H", "Content-Type: application/json", "-d", payload, url },
    });
    defer {
        allocator.free(result.stderr);
    }

    if (result.term != .Exited or result.term.Exited != 0) {
        allocator.free(result.stdout);
        return error.HttpRequestFailed;
    }

    return result.stdout;
}