//! IGLA RACE CLI Commands
//!
//! tri race init     — Initialize Neon schema + Optuna study
//! tri race start    — Start worker on current machine
//! tri race status   — Show live leaderboard
//! tri race stop     — Stop worker on current machine
//!
//! Usage:
//! ```bash
//! tri race init --neon-url $NEON_URL
//! tri race start --workers 4
//! tri race status
//! ```

const std = @import("std");

pub fn execute(args: []const []const u8) !void {
    if (args.len < 2) {
        std.debug.print("Usage: tri race <command> [args]\n", .{});
        std.debug.print("Commands:\n", .{});
        std.debug.print("  init    — Initialize Neon schema + Optuna study\n", .{});
        std.debug.print("  start   — Start worker on current machine\n", .{});
        std.debug.print("  status  — Show live leaderboard\n", .{});
        std.debug.print("  stop    — Stop worker on current machine\n", .{});
        return;
    }

    const command = args[1];

    if (std.mem.eql(u8, command, "init")) {
        try cmdRaceInit(args[2..]);
    } else if (std.mem.eql(u8, command, "start")) {
        try cmdRaceStart(args[2..]);
    } else if (std.mem.eql(u8, command, "status")) {
        try cmdRaceStatus(args[2..]);
    } else if (std.mem.eql(u8, command, "stop")) {
        try cmdRaceStop(args[2..]);
    } else {
        std.debug.print("Unknown command: {s}\n", .{command});
    }
}

fn cmdRaceInit(args: []const []const u8) !void {
    std.debug.print("[IGLA] Initializing Neon schema...\n", .{});

    const neon_url = if (args.len > 0)
        args[0]
    else
        std.os.getenv("NEON_URL") orelse {
            std.debug.print("Error: NEON_URL not set. Use --neon-url or set environment variable.\n", .{});
            return error.EnvironmentVariableNotFound;
        };

    std.debug.print("[IGLA] Neon URL: {s}\n", .{neon_url});

    // Execute SQL schema
    const result = try std.process.Child.exec(
        "psql",
        &.{neon_url, "-f", "scripts/igla_neon_schema.sql"},
        .{ .cwd = "." },
    );

    if (result.term.exit == 0) {
        std.debug.print("[IGLA] ✓ Schema initialized successfully\n", .{});
        std.debug.print("[IGLA] Next: tri race start --workers 4\n", .{});
    } else {
        std.debug.print("[IGLA] ✗ Schema initialization failed\n", .{});
        std.debug.print("{s}\n", .{result.stderr});
    }
}

fn cmdRaceStart(args: []const []const u8) !void {
    var workers: u8 = 1;
    var neon_url: ?[]const u8 = null;
    var machine_id: ?[]const u8 = null;

    for (args) |arg| {
        if (std.mem.startsWith(u8, arg, "--workers=")) {
            workers = try std.fmt.parseInt(u8, arg[11..]);
        } else if (std.mem.startsWith(u8, arg, "--neon-url=")) {
            neon_url = arg[11..];
        } else if (std.mem.startsWith(u8, arg, "--machine=")) {
            machine_id = arg[10..];
        }
    }

    if (neon_url == null) {
        neon_url = std.os.getenv("NEON_URL") orelse {
            std.debug.print("Error: NEON_URL not set\n", .{});
            return error.EnvironmentVariableNotFound;
        };
    }

    std.debug.print("[IGLA] Starting worker...\n", .{});
    std.debug.print("  Workers: {d}\n", .{workers});
    std.debug.print("  Neon URL: {s}\n", .{neon_url.?});
    if (machine_id) |id| {
        std.debug.print("  Machine ID: {s}\n", .{id});
    }

    // Start Python worker in background
    const py_cmd = try std.fmt.allocPrint(
        "python3 scripts/igla_race_worker.py --workers {d} --neon-url {s}",
        .{workers, neon_url.?}
    );

    const result = try std.process.Child.run(
        "tmux",
        &.{ "new-session", "-d", "-s", "igla_race", py_cmd },
        .{ .cwd = "." },
    );

    if (result.term.exit == 0) {
        std.debug.print("[IGLA] ✓ Worker started in tmux session 'igla_race'\n", .{});
        std.debug.print("[IGLA] View: tmux attach -t igla_race\n", .{});
        std.debug.print("[IGLA] Stop: tmux kill-session -t igla_race\n", .{});
    } else {
        std.debug.print("[IGLA] ✗ Failed to start worker\n", .{});
    }
}

fn cmdRaceStatus(args: []const []const u8) !void {
    _ = args; // unused

    const neon_url = std.os.getenv("NEON_URL") orelse {
        std.debug.print("Error: NEON_URL not set\n", .{});
        return error.EnvironmentVariableNotFound;
    };

    std.debug.print("[IGLA] Fetching race status...\n", .{});

    // Query leaderboard view
    const result = try std.process.Child.run(
        "psql",
        &.{ neon_url.?, "-c", "SELECT * FROM v_igla_leaderboard ORDER BY bpb ASC LIMIT 10;" },
        .{ .cwd = "." },
    );

    std.debug.print("{s}\n", .{result.stdout});
}

fn cmdRaceStop(args: []const []const u8) !void {
    _ = args; // unused

    std.debug.print("[IGLA] Stopping worker...\n", .{});

    const result = try std.process.Child.run(
        "tmux",
        &.{ "kill-session", "-t", "igla_race" },
        .{ .cwd = "." },
    );

    if (result.term.exit == 0) {
        std.debug.print("[IGLA] ✓ Worker stopped\n", .{});
    } else {
        std.debug.print("[IGLA] ✗ Failed to stop worker (session may not exist)\n", .{});
    }
}
