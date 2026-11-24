import assert from "node:assert/strict";
import { test } from "node:test";
import { closed, code, commandLink, escape } from "../explore/popup/markdown";

test("markdown is escaped", () => {
    assert.equal(escape("a_b.kdl › #1 [x](y) <b> $(copy)"), "a\\_b\\.kdl › \\#1 \\[x\\]\\(y\\) \\<b\\> \\$\\(copy\\)");
    assert.equal(escape("plain é"), "plain é");
});

test("code keeps no backticks", () => {
    assert.equal(code("A.B"), "`A.B`");
    assert.equal(code("a`b"), "`a'b`");
});

test("links have no parentheses", () => {
    assert.equal(closed("file:///a (1)/b"), "file:///a %281%29/b");
});

test("command links carry only numbers", () => {
    for (const [link, expected] of [[commandLink("Next", "a.b", 3), [3]], [commandLink("Next", "a.b", 3, 0), [3, 0]]] as const) {
        const [, command, query] = /^\[Next\]\(command:([^?]+)\?([^)]+)\)$/.exec(link) ?? [];

        assert.equal(command, "a.b");
        assert.deepEqual(JSON.parse(decodeURIComponent(query)), expected);
        assert.deepEqual(JSON.parse(decodeURIComponent(decodeURIComponent(query))), expected);
    }
});
