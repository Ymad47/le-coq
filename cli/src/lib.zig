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
    entity_type: EntityType,
    aliases: [][]const u8,
    properties: json.Value,
    
    pub const EntityType = enum {
        person,
        organization,
        location,
        event,
        asset,
    };
};

pub const Relationship = struct {
    id: []const u8,
    from_entity: []const u8,
    to_entity: []const u8,
    relationship_type: []const u8,
    confidence: f32,
    sources: [][]const u8,
};

/// Memory-mapped file cache for offline mode
pub const Cache = struct {
    mmap: ?std.posix.MemMap = null,
    
    pub fn init(path: []const u8) !Cache {
        // TODO: Implement memory-mapped file loading
        return Cache{};
    }
    
    pub fn deinit(self: *Cache) void {
        if (self.mmap) |mmap| {
            std.posix.munmap(mmap);
        }
    }
    
    pub fn get(self: *Cache, key: []const u8) ?[]const u8 {
        // TODO: Implement key-value lookup in mmap
        return null;
    }
};

/// Simple graph traversal
pub fn bfsNeighbors(graph: *const Graph, start: []const u8, max_depth: u32) ![]Entity {
    _ = graph;
    _ = start;
    _ = max_depth;
    // TODO: Implement breadth-first search
    return &.{};
}

pub const Graph = struct {
    entities: std.AutoHashMap([]const u8, Entity),
    relationships: std.ArrayList(Relationship),
    
    pub fn init(allocator: mem.Allocator) Graph {
        return Graph{
            .entities = std.AutoHashMap([]const u8, Entity).init(allocator),
            .relationships = std.ArrayList(Relationship).init(allocator),
        };
    }
    
    pub fn deinit(self: *Graph) void {
        self.entities.deinit();
        self.relationships.deinit();
    }
};
