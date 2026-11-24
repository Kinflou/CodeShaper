import { ExtensionContext } from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { explore } from "./explore";
import { addPreviews } from "./preview";
import { createClient } from "./server";

let client: LanguageClient | undefined;

export async function activate(context: ExtensionContext): Promise<void> {
    client = createClient(context.extensionPath);
    context.subscriptions.push(client);

    addPreviews(context, client);
    const exploring = explore(context, client);

    await client.start();
    exploring.opened();
}

export async function deactivate(): Promise<void> {
    await client?.stop();
}
