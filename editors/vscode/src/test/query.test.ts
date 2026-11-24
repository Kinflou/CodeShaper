import assert from "node:assert/strict";
import { test } from "node:test";
import { formatted } from "../explore/popup/query";

const NAMED = '((function_definition type: (primitive_type) @target declarator: (function_declarator declarator: (identifier) @name)) (#eq? @name "One"))';
const QUERIES = [
    NAMED,
    '((identifier) @target (#eq? @target "One"))',
    '((compound_statement (if_statement) @target) (#eq? @name "a  (b) ) \\" \\\\ c"))',
    "((a b: (c (d e: (f (g h: (i (j) @target))))) k: (l)* (m)+ @other) (#match? @target \"x\"))"
];

function tokens(query: string): string[] {
    return query.match(/"(?:\\.|[^"\\])*"|[()]|[^\s()"]+/g) ?? [];
}

test("long queries take several lines", () => {
    assert.equal(formatted(NAMED), [
        "(",
        "  (function_definition",
        "    type: (primitive_type) @target",
        "    declarator: (function_declarator",
        "      declarator: (identifier) @name))",
        "  (#eq? @name \"One\")",
        ")"
    ].join("\n"));
});

test("short queries stay on one line", () => {
    assert.equal(formatted('((identifier) @target  (#eq? @target "One"))'), '((identifier) @target (#eq? @target "One"))');
});

test("predicates stay on one line", () => {
    const predicate = `(#eq? @target "${"long ".repeat(20)}")`;

    assert.ok(formatted(`((a) @target ${predicate})`).split("\n").some(line => line.trim() === predicate));
});

test("only spacing changes", () => {
    for (const query of QUERIES) {
        assert.deepEqual(tokens(formatted(query)), tokens(query));
        assert.equal(formatted(formatted(query)), formatted(query));
    }
});

test("unfinished queries are laid out too", () => {
    for (const query of ["", "(", ")", "((a", "(a))", "(a \"b", "a: (b)"]) {
        assert.equal(typeof formatted(query), "string");
    }
});
