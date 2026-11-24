import { SCHEME } from "./copy";

export const COMMANDS = {
    start: "codeshaper.explore",
    next: "codeshaper.nextPlace",
    previous: "codeshaper.previousPlace",
    all: "codeshaper.allPlaces",
    highlight: "codeshaper.highlightBlock",
    parent: "codeshaper.exploreParent",
    child: "codeshaper.exploreChild",
    copy: "codeshaper.copyLocation",
    highlights: "codeshaper.toggleHighlights",
    hints: "codeshaper.toggleHints",
    edit: "codeshaper.editExplored",
    keys: "codeshaper.exploreKeys"
};

export const LINKED = [COMMANDS.next, COMMANDS.previous, COMMANDS.all, COMMANDS.parent, COMMANDS.highlight, COMMANDS.copy];

const TITLES: Record<string, string> = { "editor.action.showHover": "Show the Locations" };

export interface Manifest {
    contributes: {
        commands: { command: string; title: string }[];
        keybindings: { command: string; key: string; when: string }[];
    };
}

export interface Key {
    key: string;
    title: string;
    command: string;
}

export function keysOf({ contributes }: Manifest): Key[] {
    const titles = new Map(contributes.commands.map(({ command, title }) => [command, title]));

    return contributes.keybindings
        .filter(({ when }) => when.includes(SCHEME))
        .map(({ command, key }) => ({ key: key.toUpperCase(), title: titles.get(command) ?? TITLES[command] ?? command, command }));
}
