import assert from "node:assert/strict";
import { test } from "node:test";
import { Links } from "../explore/links";

const ASKED = { uri: "file:///p.kdl", position: { line: 3, character: 7 } };

function replaced(links: Links, text: string): string {
    return links.replaced(text, "a.open", ASKED);
}

test("links become numbered commands", () => {
    const links = new Links();

    assert.equal(
        replaced(links, "In 1\n\n[`a.cpp:4:6-15`](<file:///t/a.cpp#L4,6-L4,15>)\n- `$[0]` = `x` [`4:6-7`](<file:///t/a.cpp#L4,6-L5,7>)"),
        "In 1\n\n[`a.cpp:4:6-15`](command:a.open?%5B1%5D)\n- `$[0]` = `x` [`4:6-7`](command:a.open?%5B2%5D)"
    );
    assert.deepEqual(links.find(1), {
        asked: ASKED, uri: "file:///t/a.cpp", range: { start: { line: 3, character: 5 }, end: { line: 3, character: 14 } }
    });
    assert.deepEqual(links.find(2)?.range, { start: { line: 3, character: 5 }, end: { line: 4, character: 6 } });
    assert.equal(links.find(3), undefined);
});

test("several links on a line are told apart", () => {
    const links = new Links();

    assert.equal(
        replaced(links, "Seen as [`a`](<file:///a#L1,1-L1,2>), [`](<x>`](<file:///b%20(1)%25.cpp#L2,1-L2,1>)"),
        "Seen as [`a`](command:a.open?%5B1%5D), [`](<x>`](command:a.open?%5B2%5D)"
    );
    assert.deepEqual([links.find(1)?.uri, links.find(2)?.uri], ["file:///a", "file:///b%20(1)%25.cpp"]);
});

test("files have no range", () => {
    const links = new Links();

    replaced(links, "Matches 1 target file(s)\n\n- [`a.cpp`](<file:///t/a.cpp>)\n- [`b`](<file:///t/b#L1>)");
    assert.deepEqual([links.find(1), links.find(2)], [
        { asked: ASKED, uri: "file:///t/a.cpp", range: undefined }, { asked: ASKED, uri: "file:///t/b", range: undefined }
    ]);
});

test("other links and code stay", () => {
    const code = "[a](<file:///c#L1,1-L1,2>)";

    for (const text of [
        "[q](https://x.y/z) [f](file:///a) <file:///a>", `\`\`\`cpp\n${code}\n\`\`\``, `\`\`\`\`\n\`\`\`\n${code}\n\`\`\`\`\nend`,
        `- x\n  \`\`\`cpp\n  ${code}\n  \`\`\``, "", "plain\r\n"
    ]) {
        assert.equal(replaced(new Links(), text), text);
    }

    assert.match(replaced(new Links(), `\`\`\`cpp\nx\n\`\`\`\n${code}`), /```\n\[a\]\(command:a\.open\?%5B1%5D\)$/);
    assert.match(replaced(new Links(), `\`\`\`\n\`\`\`cpp\n${code}\n\`\`\`\n${code}`), /cpp\n\[a\]\(<file[^\n]*\n```\n\[a\]\(command/);
});

test("arguments are one number", () => {
    const links = new Links();
    const [, query] = /\(command:a\.open\?([^)]+)\)/.exec(replaced(links, "[a](<file:///a>)")) ?? [];

    assert.deepEqual(JSON.parse(decodeURIComponent(query)), [1]);
    assert.deepEqual(JSON.parse(decodeURIComponent(decodeURIComponent(query))), [1]);
});

test("old links are forgotten", () => {
    const links = new Links();

    for (let count = 0; count < 1002; count += 1) {
        replaced(links, `[a](<file:///${count}>)`);
    }

    assert.equal(links.find(2), undefined);
    assert.equal(links.find(3)?.uri, "file:///2");
    assert.equal(links.find(1002)?.uri, "file:///1001");
});
