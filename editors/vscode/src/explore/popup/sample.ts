export interface Limits {
    lines: number;
    characters: number;
}

export function sample(text: string, firstLine: string, limits: Limits): string {
    const indent = /^\s*/.exec(firstLine)?.[0] ?? "";
    const whole = text.split(/\r?\n/).map(line => line.startsWith(indent) ? line.slice(indent.length) : line);
    const lines = whole.slice(0, limits.lines).join("\n");
    const isCut = whole.length > limits.lines || lines.length > limits.characters;

    return isCut ? `${lines.slice(0, limits.characters).trimEnd()}\n…` : lines;
}
