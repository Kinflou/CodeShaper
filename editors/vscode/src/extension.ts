import { ExtensionContext } from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { explore } from "./explore";
import { Links } from "./explore/links";
import { middleware } from "./explore/middleware";
import { Requests } from "./explore/requests";
import { addPreviews } from "./preview";
import { createClient } from "./server";

let client: LanguageClient | undefined;

export async function activate(context: ExtensionContext): Promise<void> {
    const links = new Links();
    const started = createClient(context.extensionPath, middleware(links, () => requests));
    const requests = new Requests(started);

    client = started;
    context.subscriptions.push(client);

    addPreviews(context, client);
    const exploring = explore(context, requests, links);

    await client.start();
    exploring.opened();
}

export async function deactivate(): Promise<void> {
    await client?.stop();
}
