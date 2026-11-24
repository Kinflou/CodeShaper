const WIDTH = 50;

type Item = string | Item[];

export function formatted(query: string): string {
    const tokens = query.match(/"(?:\\.|[^"\\])*"|[()]|[^\s()"]+/g) ?? [];
    const items = parse(tokens);

    return items.map(item => written(item, 0)).join("\n");
}

function parse(tokens: string[]): Item[] {
    const open: Item[][] = [[]];

    for (const token of tokens) {
        if (token === "(") {
            open.push([]);
        } else if (token === ")" && open.length > 1) {
            const closed = open.pop() ?? [];
            open[open.length - 1].push(closed);
        } else {
            open[open.length - 1].push(token);
        }
    }

    while (open.length > 1) {
        const closed = open.pop() ?? [];
        open[open.length - 1].push(closed);
    }

    return open[0];
}

function written(item: Item, indent: number): string {
    const single = inline(item);
    if (typeof item === "string" || single.length <= WIDTH || single.startsWith("(#")) {
        return single;
    }

    const next = indent + 2;
    const lines: string[] = [];
    for (const [index, part] of item.entries()) {
        const previous = item[index - 1];
        const isAttached = index > 0 && typeof previous === "string" && previous.endsWith(":");
        const isCapture = index > 0 && typeof part === "string" && (part.startsWith("@") || /^[*+?]$/.test(part));

        if (isAttached || isCapture) {
            lines[lines.length - 1] += ` ${written(part, next)}`;
        } else {
            lines.push(written(part, next));
        }
    }

    const isGroup = typeof item[0] !== "string";
    const margin = `\n${" ".repeat(next)}`;

    return isGroup
        ? `(${margin}${lines.join(margin)}\n${" ".repeat(indent)})`
        : `(${lines.join(margin)})`;
}

function inline(item: Item): string {
    return typeof item === "string" ? item : `(${item.map(inline).join(" ")})`;
}
