import {
    CancellationToken, EventEmitter, InlayHint, InlayHintLabelPart, InlayHintsProvider, Position, Range, TextDocument
} from "vscode";
import { settings } from "../settings";
import { subjectOf } from "./copy";
import { Switch } from "./key";
import { popup } from "./popup";
import { BlockHint, HintPart } from "./protocol";
import { Requests } from "./requests";
import { State } from "./state";

const SEPARATOR = " › ";

export class BlockHints implements InlayHintsProvider, Switch {
    private readonly changed = new EventEmitter<void>();
    private readonly sources = new WeakMap<InlayHint, { document: TextDocument; parts: HintPart[] }>();
    private isFilled = false;
    private isShowingAll = false;
    readonly onDidChangeInlayHints = this.changed.event;

    constructor(private readonly requests: Requests, private readonly state: State) {}

    get isOn(): boolean {
        return this.isShowingAll;
    }

    turn(isOn: boolean): void {
        if (this.isShowingAll !== isOn) {
            this.isShowingAll = isOn;
            this.refresh();
        }
    }

    refresh(): void {
        this.changed.fire();
    }

    refreshFilled(): void {
        if (this.isFilled) {
            this.isFilled = false;
            this.refresh();
        }
    }

    async provideInlayHints(document: TextDocument, range: Range, token: CancellationToken): Promise<InlayHint[]> {
        const subject = subjectOf(document);
        const depth = settings.exploreDepth();
        if (!subject || (depth === 0 && !this.isShowingAll)) {
            return [];
        }

        const hints = await this.requests.hints(subject, depth, this.isShowingAll, token);

        return (hints ?? [])
            .map(hint => this.hint(document, hint))
            .filter(hint => range.contains(hint.position));
    }

    async resolveInlayHint(hint: InlayHint, token: CancellationToken): Promise<InlayHint> {
        const source = this.sources.get(hint);
        const subject = source && subjectOf(source.document);
        if (!source || !subject || typeof hint.label === "string") {
            return hint;
        }

        this.isFilled = true;
        const labels = hint.label.filter(label => label.value !== SEPARATOR);
        await Promise.all(source.parts.map(async ({ node, range }, index) => {
            const block = await this.requests.glimpse(subject, range, token, node);
            if (block) {
                labels[index].tooltip = popup(this.state, source.document, block);
            }
        }));

        return hint;
    }

    private hint(document: TextDocument, { position, parts }: BlockHint): InlayHint {
        const labels = parts.map(({ label, location }) => {
            const part = new InlayHintLabelPart(label);
            part.tooltip = location;

            return part;
        });
        const separated = labels.flatMap((label, index) => index === 0 ? [label] : [new InlayHintLabelPart(SEPARATOR), label]);

        const hint = new InlayHint(new Position(position.line, position.character), separated);
        hint.paddingRight = true;
        this.sources.set(hint, { document, parts });

        return hint;
    }
}
