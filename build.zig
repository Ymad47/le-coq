const std = @import("std");

pub fn build(b: *std.Build) void {
    // Apollo
    const apollo = b.addExecutable(.{
        .name = "apollo",
        .root_source_file = b.path("apollo/src/main.zig"),
        .target = b.standardTargetOptions(.{}),
        .optimize = b.standardOptimizeOption(.{}),
    });

    b.installArtifact(apollo);

    // Core lib
    const core = b.addStaticLibrary(.{
        .name = "core",
        .root_source_file = b.path("core/src/lib.zig"),
        .target = b.standardTargetOptions(.{}),
        .optimize = b.standardOptimizeOption(.{}),
    });

    b.installArtifact(core);

    // Run command
    const run_cmd = b.addRunArtifact(apollo);
    run_cmd.step.dependOn(b.getInstallStep());

    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const run_step = b.step("run", "Run Apollo");
    run_step.dependOn(&run_cmd.step);

    // Tests
    const core_tests = b.addTest(.{
        .root_source_file = b.path("core/src/lib.zig"),
        .target = b.standardTargetOptions(.{}),
        .optimize = b.standardOptimizeOption(.{}),
    });

    const run_core_tests = b.addRunArtifact(core_tests);
    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&run_core_tests.step);
}
