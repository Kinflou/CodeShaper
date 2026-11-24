import { PlainPosition, PlainRange } from "./protocol";

const KEPT = 1000;
const FENCE = /^\s*(`{3,})/;
const LINK = /\]\(<(file:[^<>\s]*)>\)/g;
const SPAN = /^L(\d+),(\d+)-L(\d+),(\d+)$/;

export interface Asked {
    uri: string;
    position: PlainPosition;
}

export interface Link {
    asked: Asked;
    uri: string;
    range?: PlainRange;
}

export class Links {
    private readonly links = new Map<number, Link>();
    private linked = 0;

    find(number: number): Link | undefined {
        return this.links.get(number);
    }

    replaced(text: string, command: string, asked: Asked): string {
        let fence: string | undefined;

        return text.split("\n").map(line => {
            const marker = FENCE.exec(line)?.[1];

            if (fence !== undefined) {
                fence = marker && marker.length >= fence.length && line.trim() === marker ? undefined : fence;
                return line;
            }
            if (marker) {
                fence = marker;
                return line;
            }

            return line.replace(LINK, (_, target: string) => `](command:${command}?%5B${this.remember(asked, target)}%5D)`);
        }).join("\n");
    }

    private remember(asked: Asked, target: string): number {
        const [uri, fragment = ""] = target.split("#");

        this.linked += 1;
        this.links.set(this.linked, { asked, uri, range: spanned(fragment) });
        this.links.delete(this.linked - KEPT);

        return this.linked;
    }
}

function spanned(fragment: string): PlainRange | undefined {
    const [line, character, last, end] = (SPAN.exec(fragment) ?? []).slice(1).map(found => Number(found) - 1);

    return line === undefined ? undefined : { start: { line, character }, end: { line: last, character: end } };
}
