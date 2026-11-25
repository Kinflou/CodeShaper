import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { test } from "node:test";
import { missingVariables } from "../errors";
import { ignored } from "../variables/ignored";
import { Variable } from "../variables/protocol";
import { rows, shown } from "../variables/rows";
import { Memory, ValueStore } from "../variables/store";

function memory(): Memory & { kept: Map<string, unknown> } {
    const kept = new Map<string, unknown>();

    return {
        kept,
        get: <T>(key: string) => kept.get(key) as T | undefined,
        update: async (key, value) => { kept.set(key, JSON.parse(JSON.stringify(value))); }
    };
}

function variable(name: string, value: string | null, source: Variable["source"]): Variable {
    return { name, kind: source === "constant" ? "constant" : "variable", value, source, otherwise: null, range: null };
}

test("values are kept for each project", async () => {
    const kept = memory();
    const store = new ValueStore(kept);
    assert.deepEqual(store.of("a"), {});

    await store.set("a", "prefix", "x_");
    await store.set("a", "engine", "");
    await store.set("b", "prefix", "é𝄞");
    assert.deepEqual(new ValueStore(kept).of("a"), { prefix: "x_", engine: "" });
    assert.deepEqual(store.of("b"), { prefix: "é𝄞" });

    await store.set("a", "prefix", "y_");
    await store.reset("a", "engine");
    assert.deepEqual(store.of("a"), { prefix: "y_" });
    await store.reset("a", "missing");
    await store.reset("a", "prefix");
    assert.deepEqual(kept.kept.get("codeshaper.variables"), { b: { prefix: "é𝄞" } });
});

test("given values can't be changed from outside", async () => {
    const store = new ValueStore(memory());
    await store.set("a", "prefix", "x_");
    store.of("a").prefix = "changed";

    assert.deepEqual(store.of("a"), { prefix: "x_" });
});

test("rows tell where a value comes from", () => {
    const variables = [
        variable("header", "Generated", "constant"), variable("prefix", "my_", "default"), variable("engine", "4", "configuration"),
        variable("author", "", "set"), variable("year", null, "missing")
    ];
    const found = rows({ name: "p", variables, unknown: ["gone"] });

    assert.deepEqual(found.map(({ name, description, icon, context }) => [name, description, icon, context]), [
        ["header", "Generated  (constant)", "symbol-constant", "constant"],
        ["prefix", "my_  (default)", "symbol-variable", "variable"],
        ["engine", "4  (configuration)", "symbol-variable", "variable"],
        ["author", "empty  (set here)", "symbol-variable", "set"],
        ["year", "not set", "warning", "variable"],
        ["gone", "set here, but the project has no such variable", "circle-slash", "unknown"]
    ]);
    assert.deepEqual(found.map(({ tooltip }) => tooltip), [
        "header = Generated", "prefix = my_", "engine = 4", "author = ", "year has no value", "gone isn't used"
    ]);
});

test("long values are shown on one line", () => {
    assert.equal(shown("a\r\nb\n\tc"), "a⏎b⏎⇥c");
    assert.equal(shown("𝄞".repeat(60)), "𝄞".repeat(60));
    assert.equal(shown("𝄞".repeat(61)), `${"𝄞".repeat(60)}…`);
});

test("errors name their variables", () => {
    const data = { project: "file:///p/project.kdl", variables: ["engine"] };

    assert.deepEqual(missingVariables(Object.assign(new Error("no value"), { data })), data);
    for (const other of [new Error("no"), "no", undefined, { data }, Object.assign(new Error("no"), { data: null }), Object.assign(new Error("no"), { data: { project: "a" } })]) {
        assert.equal(missingVariables(other), undefined);
    }
});

test("files git ignores are found", async () => {
    const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "é ignored ")));
    const write = (name: string, text = "") => {
        const file = path.join(root, name);
        fs.mkdirSync(path.dirname(file), { recursive: true });
        fs.writeFileSync(file, text);

        return file;
    };
    const git = (directory: string, ...args: string[]) => execFileSync("git", ["-C", path.join(root, directory), ...args], { stdio: "pipe" });

    try {
        const outside = write("outside/project.kdl");
        const shown = write("repository/a é/project.kdl");
        const tracked = write("repository/build/tracked/project.kdl");
        const hidden = write("repository/build/b\u00e9 c/project.kdl");
        const nested = write("repository/build/nested/project.kdl");
        const own = write("repository/own/project.kdl");
        const ownHidden = write("repository/own/out/project.kdl");
        write("repository/.gitignore", "/build/\n");
        write("repository/own/.gitignore", "out\n");
        git("repository", "init", "-q");
        git("repository", "add", "-f", "build/tracked/project.kdl");
        git("repository/build/nested", "init", "-q");
        git("repository/own", "init", "-q");

        const files = [outside, shown, tracked, hidden, nested, own, ownHidden];
        assert.deepEqual([...await ignored(files)].sort(), [hidden, nested, ownHidden].sort());
        assert.deepEqual(await ignored([]), new Set());

        write("broken/.git", "gitdir: missing\n");
        assert.deepEqual(await ignored([write("broken/project.kdl"), hidden]), new Set([hidden]));
    } finally {
        fs.rmSync(root, { recursive: true });
    }
});
