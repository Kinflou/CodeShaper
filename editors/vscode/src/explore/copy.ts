import {
    window, workspace, EventEmitter, FileDecorationProvider, TextDocument, TextDocumentContentProvider, TextEditor, Uri
} from "vscode";
import { ExploredFile } from "./protocol";

export const SCHEME = "codeshaper-explore";

export interface Subject {
    document: TextDocument;
    start: string;
    file: ExploredFile;
}

export const badge: FileDecorationProvider = {
    provideFileDecoration: uri => uri.scheme === SCHEME ? { badge: "⌕", tooltip: "Exploring a read-only copy" } : undefined
};

export class Copies implements TextDocumentContentProvider {
    private readonly changed = new EventEmitter<Uri>();
    readonly onDidChange = this.changed.event;

    refresh(real: Uri): void {
        workspace.textDocuments
            .filter(document => isCopy(document) && document.uri.fsPath === real.fsPath)
            .forEach(document => this.changed.fire(document.uri));
    }

    async provideTextDocumentContent(uri: Uri): Promise<string> {
        return new TextDecoder().decode(await workspace.fs.readFile(uri.with({ scheme: "file", query: "" })));
    }
}

export async function openCopy(start: string, { uri, path, language, usual }: ExploredFile): Promise<TextEditor> {
    const query = JSON.stringify({ start, file: { uri, path, language, usual } });
    const document = await workspace.openTextDocument(Uri.parse(uri).with({ scheme: SCHEME, query }));

    return window.showTextDocument(document, { preview: false });
}

export function isCopy(document: TextDocument): boolean {
    return document.uri.scheme === SCHEME;
}

export function subjectOf(document: TextDocument): Subject | undefined {
    if (!isCopy(document)) {
        return undefined;
    }

    try {
        const { start, file } = JSON.parse(document.uri.query) as Omit<Subject, "document">;

        return { document, start, file };
    } catch {
        return undefined;
    }
}

export function editorOf(document: TextDocument): TextEditor | undefined {
    return window.visibleTextEditors.find(editor => editor.document === document);
}
