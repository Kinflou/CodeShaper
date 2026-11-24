import { commands, window, Location, Range, Selection, TextDocument, TextEditor, TextEditorRevealType, Uri } from "vscode";
import { openCopy } from "./copy";
import { PlainRange } from "./protocol";
import { isSame, live } from "./ranges";
import { Requests } from "./requests";
import { State } from "./state";

const SHOWN = 5000;

export class Navigation {
    private inner: { document: TextDocument; outer: PlainRange; range: PlainRange }[] = [];

    constructor(
        private readonly requests: Requests,
        private readonly state: State,
        private readonly follow: (editor: TextEditor) => void
    ) {}

    async step(forward: boolean, popup?: number, path?: number): Promise<void> {
        const found = this.state.find(popup, path);
        const located = found && await this.state.located(found);
        if (!found || !located) {
            return;
        }

        const { editor, subject } = found;
        const { location, range } = located;
        const place = await this.requests.place(subject, range, location, forward);
        if (!place) {
            window.showInformationMessage(`${location} matches nothing in the target`);
            return;
        }

        const next = place.path === subject.file.path ? editor : await openCopy(subject, place);
        this.state.choose(next.document, { location, range: place.range });
        await this.go(next, place.range, popup);
        window.setStatusBarMessage(`${location}: place ${place.number} of ${place.places} in ${place.path}`, SHOWN);
    }

    async all(popup?: number, path?: number): Promise<void> {
        const found = this.state.find(popup, path);
        const located = found && await this.state.located(found);
        const places = located && await this.requests.places(found.subject, located.location);
        if (!places) {
            return;
        }

        const locations = places.map(place => new Location(Uri.parse(place.uri), live(place.range)));
        await commands.executeCommand("editor.action.showReferences", found.editor.document.uri, live(located.range).start, locations);
    }

    async highlight(popup?: number, path?: number): Promise<void> {
        const found = this.state.find(popup, path);
        const range = popup === undefined ? found && (await this.state.block(found))?.range : found?.range;
        if (!found || !range) {
            return;
        }

        const { editor, location } = found;
        if (popup === undefined || location === undefined) {
            await this.go(editor, range, popup);
            return;
        }

        this.state.choose(editor.document, { location, range }, popup);
        await this.go(editor, range);
        this.follow(editor);
    }

    async outwards(popup?: number): Promise<void> {
        const found = this.state.find(popup);
        const block = found && await this.state.block(found);
        const outer = block?.parents.find(parent => !isSame(parent.range, block.range))?.range;

        if (found && block && outer) {
            this.inner.push({ document: found.editor.document, outer, range: block.range });
            await this.go(found.editor, outer, popup);
        }
    }

    async inwards(): Promise<void> {
        const found = this.state.find();
        const block = found && await this.state.block(found);
        if (!found || !block) {
            return;
        }

        const left = this.inner.pop();
        const isLeft = left && left.document === found.editor.document && isSame(left.outer, block.range);
        const range = isLeft ? left.range : block.child;
        if (!isLeft) {
            this.inner = [];
        }
        if (range) {
            await this.go(found.editor, range);
        }
    }

    private async go(editor: TextEditor, range: PlainRange, popup?: number): Promise<void> {
        const shown = await window.showTextDocument(editor.document, { viewColumn: editor.viewColumn, preview: false });
        const { start, end } = live(range);

        shown.selection = new Selection(end, start);
        shown.revealRange(new Range(start, end), TextEditorRevealType.InCenterIfOutsideViewport);
        if (popup !== undefined) {
            await commands.executeCommand("editor.action.hideHover");
            await commands.executeCommand("editor.action.showHover");
        }
    }
}
