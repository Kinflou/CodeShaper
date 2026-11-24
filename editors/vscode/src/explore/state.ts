import { window, TextDocument, TextEditor } from "vscode";
import { editorOf, subjectOf, Subject } from "./copy";
import { formatted } from "./popup/query";
import { Block, BlockPath, PlainRange } from "./protocol";
import { isSame, plain } from "./ranges";
import { Requests } from "./requests";

const KEPT = 1000;

export interface Found {
    editor: TextEditor;
    subject: Subject;
    range: PlainRange;
    location?: string;
}

export interface Located {
    location: string;
    range: PlainRange;
}

interface Popup {
    document: TextDocument;
    range: PlainRange;
    paths: BlockPath[];
    locations: string[];
}

interface Chosen extends Located {
    document: TextDocument;
    path?: BlockPath;
}

export class State {
    private readonly popups = new Map<number, Popup>();
    private popped = 0;
    private chosen: Chosen | undefined;

    constructor(private readonly requests: Requests) {}

    remember(document: TextDocument, block: Block): number {
        const locations = [...block.paths.map(path => path.location), ...(block.query ? [formatted(block.query)] : [])];

        this.popped += 1;
        this.popups.set(this.popped, { document, range: block.range, paths: block.paths, locations });
        this.popups.delete(this.popped - KEPT);

        return this.popped;
    }

    find(popup?: number, path?: number): Found | undefined {
        const popped = popup === undefined ? undefined : this.popups.get(popup);
        if (popup !== undefined && !popped) {
            window.showInformationMessage("Hover the block again");
            return undefined;
        }

        const editor = popped ? editorOf(popped.document) : window.activeTextEditor;
        const subject = editor && subjectOf(editor.document);
        if (!editor || !subject) {
            window.showInformationMessage("Not exploring this file");
            return undefined;
        }

        return {
            editor, subject,
            range: popped?.range ?? plain(editor.selection),
            location: path === undefined ? undefined : popped?.locations[path]
        };
    }

    async block({ subject, range }: Found): Promise<Block | undefined> {
        const block = await this.requests.block(subject, range);
        if (block === null) {
            window.showInformationMessage("No location here");
        }

        return block ?? undefined;
    }

    async located(found: Found): Promise<Located | undefined> {
        const known = found.location ?? this.locationAt(found.editor.document, found.range);
        if (known) {
            return { location: known, range: found.range };
        }

        const block = await this.block(found);
        const path = block?.paths[0];

        return block && path && { location: path.location, range: block.range };
    }

    choose(document: TextDocument, { location, range }: Located, popup?: number): void {
        const paths = popup === undefined ? undefined : this.popups.get(popup)?.paths;

        this.chosen = { document, location, range, path: paths?.find(path => path.location === location) };
    }

    locationAt(document: TextDocument, range: PlainRange): string | undefined {
        const { chosen } = this;

        return chosen && chosen.document === document && isSame(chosen.range, range) ? chosen.location : undefined;
    }

    pathAt(document: TextDocument, block: Block): BlockPath | undefined {
        const location = this.locationAt(document, block.range);
        const known = location === this.chosen?.location ? this.chosen?.path : undefined;

        return block.paths.find(path => path.location === location) ?? known ?? block.paths[0];
    }
}
