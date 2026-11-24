import assert from "node:assert/strict";
import { test } from "node:test";
import { sample } from "../explore/popup/sample";

const LIMITS = { lines: 8, characters: 400 };

test("nested code starts at the left", () => {
    assert.equal(sample("void b()\n\t{\n\t\tint x;\n\n\t}", "\tvoid b()", LIMITS), "void b()\n{\n\tint x;\n\n}");
});

test("line endings don't matter", () => {
    assert.equal(sample("a\r\n  b\r\n  c", "  a", LIMITS), "a\nb\nc");
});

test("less indented lines are kept", () => {
    assert.equal(sample("a\n b", "    a", LIMITS), "a\n b");
});

test("long code is cut", () => {
    assert.equal(sample("a\nb\nc", "a", { lines: 2, characters: 400 }), "a\nb\n…");
    assert.equal(sample("a\nb", "a", { lines: 2, characters: 400 }), "a\nb");
    assert.equal(sample("", "", LIMITS), "");
});

test("wide code is cut", () => {
    assert.equal(sample("y".repeat(401), "y", LIMITS), `${"y".repeat(400)}\n…`);
    assert.equal(sample("y".repeat(400), "y", LIMITS), "y".repeat(400));
});
