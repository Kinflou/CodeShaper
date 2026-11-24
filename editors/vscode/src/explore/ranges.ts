import { Range } from "vscode";
import { PlainRange } from "./protocol";

export function plain({ start, end }: Range): PlainRange {
    return { start: { line: start.line, character: start.character }, end: { line: end.line, character: end.character } };
}

export function live({ start, end }: PlainRange): Range {
    return new Range(start.line, start.character, end.line, end.character);
}

export function isSame(one: PlainRange, other: PlainRange): boolean {
    return live(one).isEqual(live(other));
}
