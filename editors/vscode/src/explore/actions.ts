import { commands, env, window, Uri } from "vscode";
import { Key } from "./commands";
import { openCopy } from "./copy";
import { live } from "./ranges";
import { Requests } from "./requests";
import { State } from "./state";

const SHOWN = 5000;

export async function start(requests: Requests, uri?: string): Promise<void> {
    const target = uri ?? window.activeTextEditor?.document.uri.toString();
    const found = target && await requests.files(target);
    if (!target || !found) {
        return;
    }

    const without = found.without > 0 ? `, ${found.without} without a language` : "";
    if (found.files.length === 0) {
        window.showInformationMessage(`No file to explore${without}`);
        return;
    }

    const items = found.files.map(file => ({ label: file.path, description: file.language, file }));
    const chosen = items.length === 1 ? items[0] : await window.showQuickPick(items, { placeHolder: `${items.length} file(s)${without}` });
    if (chosen) {
        await openCopy(target, chosen.file);
    }
}

export async function copy(state: State, popup?: number, path?: number): Promise<void> {
    const found = state.find(popup, path);
    const located = found && await state.located(found);

    if (located) {
        await env.clipboard.writeText(located.location);
        window.setStatusBarMessage(`Copied ${located.location.replace(/\s+/g, " ")}`, SHOWN);
    }
}

export async function edit(state: State): Promise<void> {
    const found = state.find();

    if (found) {
        await window.showTextDocument(Uri.parse(found.subject.file.uri), { preview: false, selection: live(found.range) });
    }
}

export async function showKeys(keys: Key[]): Promise<void> {
    const items = keys.map(({ key, title, command }) => ({ label: title, description: key, command }));
    const chosen = await window.showQuickPick(items, { placeHolder: "Keys while exploring" });

    if (chosen) {
        await commands.executeCommand(chosen.command);
    }
}
