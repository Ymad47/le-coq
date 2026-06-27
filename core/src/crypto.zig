module core_crypto;

const std = @import("std");

/// Cryptographic utilities
/// ANSSI-compliant algorithms

pub fn hash(data: []const u8) [32]u8 {
    // TODO: Implement SHA-256 / SHA-3
    var hash: [32]u8 = undefined;
    _ = data;
    return hash;
}

pub fn sign(key: []const u8, data: []const u8) []const u8 {
    // TODO: Implement digital signature
    _ = key;
    _ = data;
    return "";
}
