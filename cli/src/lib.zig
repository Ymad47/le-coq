const std = @import("std");
const mem = std.mem;
const fs = std.fs;
const json = std.json;

pub const Config = struct {
    backend_url: []const u8,
    offline_mode: bool,
    cache_dir: []const u8,
    
    pub fn load() !Config {
        // Load from ~/.le-coq/config.json or env vars
        return Config{
            .backend_url = "http://localhost:8000",
            .offline_mode = false,
            .cache_dir = "~/.le-coq/cache",
        };
    }
};

pub const Entity = struct {
    id: []const u8,
    name: []const u8,
    entity_type: []const u8,
    properties: json.Value,
};

pub const SearchResponse = struct {
    count: usize,
    query: []const u8,
    results: []Entity,
};

pub const ListResponse = struct {
    entities: []Entity,
};