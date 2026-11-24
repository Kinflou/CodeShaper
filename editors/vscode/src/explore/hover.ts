import { CancellationToken, Hover, HoverProvider, Position, Range, TextDocument } from "vscode";
import { editorOf, subjectOf } from "./copy";
import { popup } from "./popup";
import { live, plain } from "./ranges";
import { Requests } from "./requests";
import { State } from "./state";

export class BlockHover implements HoverProvider {
    constructor(private readonly requests: Requests, private readonly state: State) {}

    async provideHover(document: TextDocument, position: Position, token: CancellationToken): Promise<Hover | undefined> {
        const subject = subjectOf(document);
        if (!subject) {
            return undefined;
        }

        const selection = editorOf(document)?.selection;
        const range = selection && !selection.isEmpty && selection.contains(position) ? selection : new Range(position, position);
        const block = await this.requests.glimpse(subject, plain(range), token);

        return block ? new Hover(popup(this.state, document, block), live(block.range)) : undefined;
    }
}
