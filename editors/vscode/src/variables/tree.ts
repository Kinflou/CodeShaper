import { Event, EventEmitter, ThemeIcon, TreeDataProvider, TreeItem, TreeItemCollapsibleState, Uri, workspace } from "vscode";
import { ProjectVariables, Variable } from "./protocol";
import { Row, rows } from "./rows";
import { COMMANDS } from "./commands";

export interface Project {
    uri: string;
    found: ProjectVariables;
}

export interface Entry {
    project: Project;
    row: Row;
    variable?: Variable;
}

export type Node = Project | Entry;

export class VariableTree implements TreeDataProvider<Node> {
    private readonly changed = new EventEmitter<void>();
    readonly onDidChangeTreeData: Event<void> = this.changed.event;

    projects: Project[] = [];

    show(projects: Project[]): void {
        this.projects = projects;
        this.changed.fire();
    }

    getTreeItem(node: Node): TreeItem {
        return "row" in node ? entryItem(node) : projectItem(node);
    }

    getChildren(node?: Node): Node[] {
        if (!node) {
            return this.projects;
        }

        return "row" in node ? [] : entries(node);
    }
}

export function entries(project: Project): Entry[] {
    const variables = new Map(project.found.variables.map(variable => [variable.name, variable]));

    return rows(project.found).map(row => ({ project, row, variable: row.context === "unknown" ? undefined : variables.get(row.name) }));
}

function projectItem(project: Project): TreeItem {
    const { name, variables, unknown } = project.found;
    const isEmpty = variables.length === 0 && unknown.length === 0;
    const item = new TreeItem(name || "Project", isEmpty ? TreeItemCollapsibleState.None : TreeItemCollapsibleState.Expanded);

    item.id = project.uri;
    item.description = isEmpty ? "no variables" : undefined;
    item.tooltip = workspace.asRelativePath(Uri.parse(project.uri));
    item.contextValue = "project";

    return item;
}

function entryItem(entry: Entry): TreeItem {
    const { name, description, tooltip, icon, context } = entry.row;
    const item = new TreeItem(name);

    item.id = `${entry.project.uri}#${context === "unknown" ? "unknown" : "known"}#${name}`;
    item.description = description;
    item.tooltip = tooltip;
    item.iconPath = new ThemeIcon(icon);
    item.contextValue = context;
    if (context !== "unknown") {
        item.command = { command: COMMANDS.open, title: "Go to Declaration", arguments: [entry] };
    }

    return item;
}
