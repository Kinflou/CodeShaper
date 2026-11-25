import * as fs from "fs";
import { commands, Disposable, ExtensionContext, Range, Uri, window, workspace } from "vscode";
import { LanguageClient, State } from "vscode-languageclient/node";
import { COMMANDS } from "./commands";
import { ignored } from "./ignored";
import { MissingVariables, ProjectVariables } from "./protocol";
import { ValueStore } from "./store";
import { entries, Entry, Project, VariableTree } from "./tree";

const VIEW = "codeshaper.variables";
const SEARCHED = "codeshaper.searched";
const PROJECTS = "**/project.kdl";
const DOCUMENTS = "**/{*.kdl,.gitignore}";
const DELAY = 200;

export class Variables {
    private readonly tree = new VariableTree();
    private readonly store: ValueStore;
    private asked = 0;
    private hasSearched = false;
    private timer: NodeJS.Timeout | undefined;

    constructor(context: ExtensionContext, private readonly client: LanguageClient) {
        this.store = new ValueStore(context.workspaceState);
        const watcher = workspace.createFileSystemWatcher(DOCUMENTS);

        context.subscriptions.push(
            window.registerTreeDataProvider(VIEW, this.tree),
            commands.registerCommand(COMMANDS.set, (entry?: Entry) => entry && this.set(entry)),
            commands.registerCommand(COMMANDS.reset, (entry?: Entry) => entry && this.reset(entry)),
            commands.registerCommand(COMMANDS.open, (entry?: Entry) => entry && open(entry)),
            client.onDidChangeState(({ newState }) => { if (newState === State.Running) { void this.start(); } }),
            watcher,
            watcher.onDidCreate(() => this.later()),
            watcher.onDidChange(() => this.later()),
            watcher.onDidDelete(() => this.later()),
            new Disposable(() => clearTimeout(this.timer))
        );
    }

    async refresh(): Promise<void> {
        const asked = ++this.asked;
        const uris = await this.shown(await found());

        for (const uri of uris) {
            await this.send(uri);
        }
        const listed = await Promise.all(uris.map(uri => this.list(uri)));
        if (asked !== this.asked) {
            return;
        }

        const projects = listed.filter(project => project !== undefined);
        this.tree.show(projects);
        this.hasSearched = true;
        await commands.executeCommand("setContext", SEARCHED, true);
    }

    async askFor(missing: MissingVariables): Promise<boolean> {
        const wanted = real(missing.project);
        const uri = (await found()).find(uri => real(uri) === wanted);
        const project = uri === undefined ? undefined : await this.list(uri);

        for (const name of missing.variables) {
            const entry = project && entries(project).find(entry => entry.variable?.name === name);
            if (!entry || !await this.set(entry)) {
                return false;
            }
        }

        return true;
    }

    private async start(): Promise<void> {
        if (this.hasSearched) {
            return this.refresh();
        }

        await window.withProgress({ location: { viewId: VIEW } }, () => this.refresh());
    }

    private async set({ project, row: { name }, variable }: Entry): Promise<boolean> {
        const value = await window.showInputBox({
            title: `${name} (${project.found.name})`,
            prompt: "The value for previews and hovers in this workspace, runs don't use it",
            value: variable?.value ?? "",
            ignoreFocusOut: true
        });
        if (value === undefined) {
            return false;
        }

        await this.store.set(project.uri, name, value);
        await this.refresh();

        return true;
    }

    private async reset({ project, row: { name } }: Entry): Promise<void> {
        await this.store.reset(project.uri, name);
        await this.refresh();
    }

    private async shown(uris: string[]): Promise<string[]> {
        const hidden = await ignored(uris.map(real));

        return uris.filter(uri => !hidden.has(real(uri)) || Object.keys(this.store.of(uri)).length > 0);
    }

    private later(): void {
        clearTimeout(this.timer);
        this.timer = setTimeout(() => void this.refresh(), DELAY);
    }

    private async send(project: string): Promise<void> {
        try {
            await this.client.sendNotification("codeshaper/setVariables", { project, values: this.store.of(project) });
        } catch {
            return;
        }
    }

    private async list(uri: string): Promise<Project | undefined> {
        try {
            return { uri, found: await this.client.sendRequest<ProjectVariables>("codeshaper/variables", { project: uri }) };
        } catch {
            return undefined;
        }
    }
}

async function found(): Promise<string[]> {
    const files = await workspace.findFiles(PROJECTS);

    return [...new Set(files.map(file => file.toString()))].sort();
}

async function open({ project, variable }: Entry): Promise<void> {
    const range = variable?.range;
    const selection = range ? new Range(range.start.line, range.start.character, range.end.line, range.end.character) : undefined;

    await window.showTextDocument(Uri.parse(project.uri), { selection });
}

function real(uri: string): string {
    const path = Uri.parse(uri).fsPath;

    try {
        return fs.realpathSync.native(path);
    } catch {
        return path;
    }
}
