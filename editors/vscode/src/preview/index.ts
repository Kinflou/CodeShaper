import * as path from "path";
import { commands, window, workspace, ExtensionContext, Uri } from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { named } from "../arguments";
import { missingVariables, reason } from "../errors";
import { Variables } from "../variables";
import { PreviewContents, SCHEME } from "./contents";
import { OperationResult, PatchNames, PreviewedPatch, PreviewFile, PreviewResult } from "./protocol";

const SET_VALUE = "Set Value";

interface Server {
    client: LanguageClient;
    variables: Variables;
}

export function addPreviews(context: ExtensionContext, client: LanguageClient, variables: Variables): void {
    const contents = new PreviewContents();
    const server = { client, variables };

    context.subscriptions.push(
        workspace.registerTextDocumentContentProvider(SCHEME, contents),
        commands.registerCommand("codeshaper.preview", (resource?: Uri | string, name?: unknown) => previewPatch(server, contents, resource?.toString(), named(name))),
        commands.registerCommand("codeshaper.previewOperation", (resource?: Uri | string) => previewOperation(server, contents, resource?.toString()))
    );
}

async function previewPatch(server: Server, contents: PreviewContents, uri?: string, name?: string): Promise<void> {
    const target = uri ?? active();
    const found = target && await previewed(server, name ? { uri: target, name } : { uri: target, position: cursor(target) });
    if (!target || !found) {
        return;
    }
    const { result } = found;

    warn(result, count => `CodeShaper could not preview ${count} file(s)`);
    if (result.files.length === 0) {
        window.showInformationMessage(`${result.matched} file(s) matched, nothing would change`);
        return;
    }

    const chosen = result.files.length === 1 ? result.files[0] : await pick(result.files);
    if (!chosen) {
        return;
    }

    const patch = found.name ?? path.basename(Uri.parse(target).path);
    const preview = contents.add(chosen, patch);

    await commands.executeCommand("vscode.diff", Uri.parse(chosen.uri), preview, `${path.basename(chosen.path)} ↔ ${patch} preview`);
}

async function previewed(server: Server, patch: PreviewedPatch): Promise<{ result: PreviewResult; name?: string } | undefined> {
    const result = await send<PreviewResult | PatchNames>(server, "codeshaper/preview", patch);
    if (!result) {
        return undefined;
    }
    if (!("patches" in result)) {
        return { result, name: result.patch ?? undefined };
    }

    const name = await window.showQuickPick(result.patches, { placeHolder: "The patch to preview" });

    return name ? previewed(server, { uri: patch.uri, name }) : undefined;
}

async function previewOperation(server: Server, contents: PreviewContents, uri?: string): Promise<void> {
    const target = uri ?? active();
    const result = target && await send<OperationResult>(server, "codeshaper/previewOperation", { uri: target });
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

function cursor(uri: string): PreviewedPatch["position"] {
    const editor = window.activeTextEditor;
    if (editor?.document.uri.toString() !== uri) {
        return undefined;
    }
    const { line, character } = editor.selection.active;

    return { line, character };
}

async function send<T>(server: Server, method: string, params: object): Promise<T | undefined> {
    try {
        return await server.client.sendRequest<T>(method, params);
    } catch (error) {
        const missing = missingVariables(error);
        const actions = missing ? [SET_VALUE] : [];
        const chosen = await window.showErrorMessage(`CodeShaper preview failed: ${reason(error)}`, ...actions);

        return missing && chosen && await server.variables.askFor(missing) ? send(server, method, params) : undefined;
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
