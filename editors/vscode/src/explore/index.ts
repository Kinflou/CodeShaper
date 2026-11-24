import { commands, languages, window, workspace, ExtensionContext, TextEditor, Uri } from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { named } from "../arguments";
import { SECTION } from "../settings";
import { copy, edit, showKeys, start } from "./actions";
import { keysOf, COMMANDS } from "./commands";
import { badge, isCopy, Copies, SCHEME } from "./copy";
import { Cursor } from "./cursor";
import { BlockHints } from "./hints";
import { BlockHover } from "./hover";
import { HeldKey, CLOCK } from "./key";
import { Navigation } from "./navigation";
import { Requests } from "./requests";
import { State } from "./state";

const EXPLORING = "codeshaper.exploring";

export interface Exploring {
    opened(): void;
}

export function explore(context: ExtensionContext, client: LanguageClient): Exploring {
    const requests = new Requests(client);
    const state = new State(requests);
    const copies = new Copies();
    const hints = new BlockHints(requests, state);
    const cursor = new Cursor(requests, state);
    const navigation = new Navigation(requests, state, editor => cursor.follow(editor));
    const key = new HeldKey(hints, CLOCK);
    const keys = keysOf(context.extension.packageJSON);

    const activated = (editor: TextEditor | undefined) => {
        const isExploring = editor !== undefined && isCopy(editor.document);
        commands.executeCommand("setContext", EXPLORING, isExploring);

        if (isExploring) {
            copies.refresh(editor.document.uri);
            cursor.enter(editor);
        } else {
            cursor.leave();
        }
    };

    const actions: Record<string, (...given: never[]) => unknown> = {
        [COMMANDS.start]: (resource?: Uri | string, patch?: unknown) => start(requests, resource?.toString(), named(patch)),
        [COMMANDS.next]: (popup?: number, path?: number) => navigation.step(true, popup, path),
        [COMMANDS.previous]: (popup?: number, path?: number) => navigation.step(false, popup, path),
        [COMMANDS.all]: (popup?: number, path?: number) => navigation.all(popup, path),
        [COMMANDS.highlight]: (popup?: number, path?: number) => navigation.highlight(popup, path),
        [COMMANDS.parent]: (popup?: number) => navigation.outwards(popup),
        [COMMANDS.child]: () => navigation.inwards(),
        [COMMANDS.copy]: (popup?: number, path?: number) => copy(state, popup, path),
        [COMMANDS.highlights]: () => cursor.toggleHighlights(),
        [COMMANDS.hints]: () => key.press(),
        [COMMANDS.edit]: () => edit(state),
        [COMMANDS.keys]: () => showKeys(keys)
    };

    context.subscriptions.push(
        cursor, key,
        workspace.registerTextDocumentContentProvider(SCHEME, copies),
        languages.registerHoverProvider({ scheme: SCHEME }, new BlockHover(requests, state)),
        languages.registerInlayHintsProvider({ scheme: SCHEME }, hints),
        window.registerFileDecorationProvider(badge),
        window.onDidChangeActiveTextEditor(activated),
        window.onDidChangeTextEditorSelection(({ textEditor }) => {
            if (textEditor === window.activeTextEditor) {
                cursor.follow(textEditor);
            }
            if (isCopy(textEditor.document)) {
                hints.refreshFilled();
            }
        }),
        workspace.onDidSaveTextDocument(document => {
            copies.refresh(document.uri);
            if (document.languageId === "kdl") {
                hints.refresh();
            }
        }),
        workspace.onDidChangeConfiguration(change => {
            if (change.affectsConfiguration(`${SECTION}.explore`)) {
                hints.refresh();
            }
        }),
        ...Object.entries(actions).map(([command, run]) => commands.registerCommand(command, run))
    );

    return {
        opened: () => {
            activated(window.activeTextEditor);
            hints.refresh();
        }
    };
}
