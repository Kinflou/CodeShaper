import {
    window, CancellationTokenSource, Disposable, OverviewRulerLane, StatusBarAlignment, TextEditor, ThemeColor
} from "vscode";
import { COMMANDS } from "./commands";
import { isCopy, subjectOf } from "./copy";
import { Block } from "./protocol";
import { live, plain } from "./ranges";
import { Requests } from "./requests";
import { State } from "./state";

const DELAY = 100;
const IDLE = "$(telescope) Exploring";

export class Cursor implements Disposable {
    private readonly status = window.createStatusBarItem(StatusBarAlignment.Right, 100);
    private readonly blockColor = window.createTextEditorDecorationType({
        backgroundColor: new ThemeColor("editor.wordHighlightStrongBackground")
    });
    private readonly placeColor = window.createTextEditorDecorationType({
        backgroundColor: new ThemeColor("editor.wordHighlightBackground"),
        overviewRulerColor: new ThemeColor("editorOverviewRuler.wordHighlightForeground"),
        overviewRulerLane: OverviewRulerLane.Center
    });
    private following: CancellationTokenSource | undefined;
    private timer: NodeJS.Timeout | undefined;
    private isHighlighting = false;

    constructor(private readonly requests: Requests, private readonly state: State) {
        this.status.command = COMMANDS.keys;
    }

    enter(editor: TextEditor): void {
        this.status.text = IDLE;
        this.status.tooltip = "Show the keys for exploring";
        this.status.show();
        this.follow(editor);
    }

    leave(): void {
        this.status.hide();
    }

    follow(editor: TextEditor): void {
        const subject = subjectOf(editor.document);
        if (!subject) {
            return;
        }

        clearTimeout(this.timer);
        this.following?.cancel();
        const following = new CancellationTokenSource();
        this.following = following;

        this.timer = setTimeout(async () => {
            const block = await this.requests.glimpse(subject, plain(editor.selection), following.token);
            if (!following.token.isCancellationRequested && block !== undefined) {
                this.mark(editor, block);
            }
        }, DELAY);
    }

    toggleHighlights(): void {
        this.isHighlighting = !this.isHighlighting;

        for (const editor of window.visibleTextEditors.filter(editor => isCopy(editor.document))) {
            if (editor === window.activeTextEditor && this.isHighlighting) {
                this.follow(editor);
            } else if (!this.isHighlighting) {
                editor.setDecorations(this.blockColor, []);
                editor.setDecorations(this.placeColor, []);
            }
        }
    }

    dispose(): void {
        clearTimeout(this.timer);
        this.status.dispose();
        this.blockColor.dispose();
        this.placeColor.dispose();
    }

    private mark(editor: TextEditor, block: Block | null): void {
        const path = block ? this.state.pathAt(editor.document, block) : undefined;
        const isShown = this.isHighlighting && block !== null;

        this.status.text = path ? `$(telescope) ${path.location} ${path.number ?? "?"}/${path.places}` : IDLE;
        editor.setDecorations(this.blockColor, isShown ? [live(block.range)] : []);
        editor.setDecorations(this.placeColor, isShown
            ? (path?.ranges ?? []).map(live).filter(range => !range.isEqual(live(block.range)))
            : []);
    }
}
