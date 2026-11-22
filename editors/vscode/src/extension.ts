import { workspace, ExtensionContext } from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: ExtensionContext): Promise<void> {
    const command = workspace.getConfiguration("codeshaper").get<string>("server.path", "codeshaper");

    const serverOptions: ServerOptions = { command, args: ["lsp"] };
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "kdl" }]
    };

    client = new LanguageClient("codeshaper", "CodeShaper", serverOptions, clientOptions);
    context.subscriptions.push(client);

    await client.start();
}

export async function deactivate(): Promise<void> {
    await client?.stop();
}
