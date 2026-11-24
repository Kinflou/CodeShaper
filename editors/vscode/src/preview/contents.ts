import { EventEmitter, TextDocumentContentProvider, Uri } from "vscode";
import { PreviewFile } from "./protocol";

export const SCHEME = "codeshaper-preview";

export class PreviewContents implements TextDocumentContentProvider {
    private readonly contents = new Map<string, string>();
    private readonly changed = new EventEmitter<Uri>();
    readonly onDidChange = this.changed.event;

    add(file: PreviewFile, query: string): Uri {
        const uri = Uri.from({ scheme: SCHEME, path: Uri.parse(file.uri).path, query });

        this.contents.set(uri.toString(), file.result);
        this.changed.fire(uri);

        return uri;
    }

    provideTextDocumentContent(uri: Uri): string {
        return this.contents.get(uri.toString()) ?? "";
    }
}
