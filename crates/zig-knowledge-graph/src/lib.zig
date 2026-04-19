const std = @import("std");

pub const NodeId = u64;

/// Graph node
pub const Node = struct {
    id: NodeId,
    data: []const u8,

    pub fn init(id: NodeId, data: []const u8) Node {
        return .{
            .id = id,
            .data = data,
        };
    }
};

/// Graph edge
pub const Edge = struct {
    from: NodeId,
    to: NodeId,
    weight: f32,

    pub fn init(from: NodeId, to: NodeId, weight: f32) Edge {
        return .{
            .from = from,
            .to = to,
            .weight = weight,
        };
    }
};

/// Knowledge graph (R0 stub)
pub const Graph = struct {
    allocator: std.mem.Allocator,
    nodes: std.AutoHashMap(NodeId, []const u8),
    edges: std.ArrayList(Edge),
    next_id: NodeId,

    pub fn init(allocator: std.mem.Allocator) Graph {
        return .{
            .allocator = allocator,
            .nodes = std.AutoHashMap(NodeId, []const u8).init(allocator),
            .edges = std.ArrayList(Edge).init(allocator),
            .next_id = 1,
        };
    }

    pub fn deinit(self: *Graph) void {
        self.nodes.deinit();
        self.edges.deinit();
    }

    pub fn addNode(self: *Graph, data: []const u8) !NodeId {
        const id = self.next_id;
        self.next_id += 1;
        try self.nodes.put(id, data);
        return id;
    }

    pub fn addEdge(self: *Graph, from: NodeId, to: NodeId, weight: f32) !void {
        if (!self.nodes.contains(from) or !self.nodes.contains(to)) {
            return error.NodeNotFound;
        }
        try self.edges.append(.{ .from = from, .to = to, .weight = weight });
    }

    pub fn getNeighbors(self: *const Graph, id: NodeId, allocator: std.mem.Allocator) ![]NodeId {
        var list = std.ArrayList(NodeId).init(allocator);
        for (self.edges.items) |edge| {
            if (edge.from == id) {
                try list.append(edge.to);
            }
        }
        return list.toOwnedSlice();
    }

    pub fn nodeCount(self: *const Graph) usize {
        return self.nodes.count();
    }

    pub fn edgeCount(self: *const Graph) usize {
        return self.edges.items.len;
    }
};

/// C-ABI exports for TRIOS FFI
export fn kg_graph_create() *Graph {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const allocator = gpa.allocator();
    const graph = allocator.create(Graph) catch return null;
    graph.* = Graph.init(allocator);
    return graph;
}

export fn kg_graph_destroy(ptr: *Graph) void {
    if (ptr) |g| {
        g.deinit();
    }
}

export fn kg_graph_add_node(ptr: *Graph, data: [*]const u8, len: usize) NodeId {
    if (ptr) |g| {
        const slice = data[0..len];
        return g.addNode(slice) catch 0;
    }
    return 0;
}

export fn kg_graph_add_edge(ptr: *Graph, from: NodeId, to: NodeId, weight: f32) i32 {
    if (ptr) |g| {
        g.addEdge(from, to, weight) catch return -1;
        return 0;
    }
    return -1;
}

export fn kg_graph_node_count(ptr: *const Graph) usize {
    if (ptr) |g| {
        return g.nodeCount();
    }
    return 0;
}

export fn kg_graph_edge_count(ptr: *const Graph) usize {
    if (ptr) |g| {
        return g.edgeCount();
    }
    return 0;
}
