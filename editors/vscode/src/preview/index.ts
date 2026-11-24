import * as path from "path";
import { commands, window, workspace, ExtensionContext, Uri } from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { reason } from "../errors";
import { PreviewContents, SCHEME } from "./contents";
import { OperationResult, PreviewFile, PreviewResult } from "./protocol";

export function addPreviews(context: ExtensionContext, client: LanguageClient): void {
    const contents = new PreviewContents();

    context.subscriptions.push(
        workspace.registerTextDocumentContentProvider(SCHEME, contents),
        commands.registerCommand("codeshaper.preview", (resource?: Uri | string) => previewPatch(client, contents, resource?.toString())),
        commands.registerCommand("codeshaper.previewOperation", (resource?: Uri | string) => previewOperation(client, contents, resource?.toString()))
    );
}

async function previewPatch(client: LanguageClient, contents: PreviewContents, uri?: string): Promise<void> {
    const target = uri ?? active();
    const result = target && await send<PreviewResult>(client, "codeshaper/preview", target);
    if (!target || !result) {
        return;
    }

    warn(result, count => `CodeShaper could not preview ${count} file(s)`);
    if (result.files.length === 0) {
        window.showInformationMessage(`${result.matched} file(s) matched, nothing would change`);
        return;
    }

    const chosen = result.files.length === 1 ? result.files[0] : await pick(result.files);
    if (!chosen) {
        return;
    }

    const patch = path.basename(Uri.parse(target).path);
    const previewed = contents.add(chosen, patch);

    await commands.executeCommand("vscode.diff", Uri.parse(chosen.uri), previewed, `${path.basename(chosen.path)} ↔ ${patch} preview`);
}

async function previewOperation(client: LanguageClient, contents: PreviewContents, uri?: string): Promise<void> {
    const target = uri ?? active();
    const result = target && await send<OperationResult>(client, "codeshaper/previewOperation", target);
    if (!result) {
        return;
    }

    warn(result, count => `CodeShaper could not apply ${count} patch(es), a run would write nothing`);
    if (result.files.length === 0) {
        window.showInformationMessage("Nothing would change");
        return;
    }

    const changes = result.files.map(file => {
        const original = Uri.parse(file.uri);

        return [original, original, contents.add(file, "operation")];
    });

    await commands.executeCommand("vscode.changes", `Operation preview (${result.files.length} file(s))`, changes);
}

function active(): string | undefined {
    return window.activeTextEditor?.document.uri.toString();
}

async function send<T>(client: LanguageClient, method: string, uri: string): Promise<T | undefined> {
    try {
        return await client.sendRequest<T>(method, { uri });
    } catch (error) {
        window.showErrorMessage(`CodeShaper preview failed: ${reason(error)}`);
        return undefined;
    }
}

function warn(result: OperationResult, title: (count: number) => string): void {
    if (result.errors.length > 0) {
        const failed = result.errors.map(error => `${error.path}: ${error.message}`).join("; ");
        window.showWarningMessage(`${title(result.errors.length)}: ${failed}`);
    }
}

async function pick(files: PreviewFile[]): Promise<PreviewFile | undefined> {
    const items = files.map(file => ({ label: path.basename(file.path), description: file.path, file }));
    const chosen = await window.showQuickPick(items, { placeHolder: `${files.length} files would change` });

    return chosen?.file;
}
