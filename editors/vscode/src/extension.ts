import * as fs from "fs";
import * as path from "path";
import {
    commands, window, workspace, EventEmitter, ExtensionContext, TextDocumentContentProvider, Uri
} from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";

const PREVIEW_SCHEME = "codeshaper-preview";

interface PreviewFile {
    uri: string;
    path: string;
    before: string;
    result: string;
}

interface PreviewError {
    path: string;
    message: string;
}

interface PreviewResult {
    matched: number;
    files: PreviewFile[];
    errors: PreviewError[];
}

interface OperationResult {
    files: PreviewFile[];
    errors: PreviewError[];
}

let client: LanguageClient | undefined;

class PreviewProvider implements TextDocumentContentProvider {
    private readonly contents = new Map<string, string>();
    private readonly changed = new EventEmitter<Uri>();
    readonly onDidChange = this.changed.event;

    set(uri: Uri, content: string): void {
        this.contents.set(uri.toString(), content);
        this.changed.fire(uri);
    }

    provideTextDocumentContent(uri: Uri): string {
        return this.contents.get(uri.toString()) ?? "";
    }
}

function serverCommand(configured: string, extensionPath: string): string {
    const name = process.platform === "win32" ? "codeshaper.exe" : "codeshaper";
    const bundled = path.join(extensionPath, "server", name);

    return configured.trim() || (fs.existsSync(bundled) ? bundled : "codeshaper");
}

export async function activate(context: ExtensionContext): Promise<void> {
    const configuration = workspace.getConfiguration("codeshaper");
    const command = serverCommand(configuration.get<string>("server.path", ""), context.extensionPath);

    const serverOptions: ServerOptions = { command, args: ["lsp"] };
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "kdl" }],
        initializationOptions: {
            hover: {
                samples: configuration.get<number>("hover.samples", 3),
                lines: configuration.get<number>("hover.lines", 8),
                characters: configuration.get<number>("hover.characters", 400)
            }
        }
    };

    client = new LanguageClient("codeshaper", "CodeShaper", serverOptions, clientOptions);
    context.subscriptions.push(client);

    const previews = new PreviewProvider();
    context.subscriptions.push(
        workspace.registerTextDocumentContentProvider(PREVIEW_SCHEME, previews),
        commands.registerCommand("codeshaper.preview", (resource?: Uri | string) => preview(previews, resource?.toString())),
        commands.registerCommand("codeshaper.previewOperation", (resource?: Uri | string) => previewOperation(previews, resource?.toString()))
    );

    await client.start();
}

export async function deactivate(): Promise<void> {
    await client?.stop();
}

async function preview(previews: PreviewProvider, uri?: string): Promise<void> {
    const target = uri ?? window.activeTextEditor?.document.uri.toString();
    if (!client || !target) {
        return;
    }

    let result: PreviewResult;
    try {
        result = await client.sendRequest<PreviewResult>("codeshaper/preview", { uri: target });
    } catch (error) {
        window.showErrorMessage(`CodeShaper preview failed: ${error instanceof Error ? error.message : String(error)}`);
        return;
    }

    if (result.errors.length > 0) {
        const failed = result.errors.map(error => `${error.path}: ${error.message}`).join("; ");
        window.showWarningMessage(`CodeShaper could not preview ${result.errors.length} file(s): ${failed}`);
    }

    if (result.files.length === 0) {
        window.showInformationMessage(`${result.matched} file(s) matched, nothing would change`);
        return;
    }

    const chosen = result.files.length === 1 ? result.files[0] : await pick(result.files);
    if (!chosen) {
        return;
    }

    const patch = path.basename(Uri.parse(target).path);
    const previewUri = Uri.from({ scheme: PREVIEW_SCHEME, path: Uri.parse(chosen.uri).path, query: patch });
    previews.set(previewUri, chosen.result);

    const title = `${path.basename(chosen.path)} ↔ ${patch} preview`;
    await commands.executeCommand("vscode.diff", Uri.parse(chosen.uri), previewUri, title);
}

async function previewOperation(previews: PreviewProvider, uri?: string): Promise<void> {
    const target = uri ?? window.activeTextEditor?.document.uri.toString();
    if (!client || !target) {
        return;
    }

    let result: OperationResult;
    try {
        result = await client.sendRequest<OperationResult>("codeshaper/previewOperation", { uri: target });
    } catch (error) {
        window.showErrorMessage(`CodeShaper preview failed: ${error instanceof Error ? error.message : String(error)}`);
        return;
    }

    if (result.errors.length > 0) {
        const failed = result.errors.map(error => `${error.path}: ${error.message}`).join("; ");
        window.showWarningMessage(`CodeShaper could not apply ${result.errors.length} patch(es), a run would write nothing: ${failed}`);
    }

    if (result.files.length === 0) {
        window.showInformationMessage("Nothing would change");
        return;
    }

    const changes = result.files.map(file => {
        const original = Uri.parse(file.uri);
        const previewUri = Uri.from({ scheme: PREVIEW_SCHEME, path: original.path, query: "operation" });
        previews.set(previewUri, file.result);

        return [original, original, previewUri];
    });

    await commands.executeCommand("vscode.changes", `Operation preview (${result.files.length} file(s))`, changes);
}

async function pick(files: PreviewFile[]): Promise<PreviewFile | undefined> {
    const items = files.map(file => ({ label: path.basename(file.path), description: file.path, file }));
    const chosen = await window.showQuickPick(items, { placeHolder: `${files.length} files would change` });

    return chosen?.file;
}
