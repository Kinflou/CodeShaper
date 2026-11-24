import { workspace } from "vscode";

export const SECTION = "codeshaper";

export interface HoverSettings {
    samples: number;
    lines: number;
    characters: number;
}

export interface HintSettings {
    groups: boolean;
    matches: boolean;
}

export const settings = {
    serverPath: (): string => read("server.path", ""),
    hover: (): HoverSettings => ({
        samples: read("hover.samples", 3),
        lines: read("hover.lines", 8),
        characters: read("hover.characters", 400)
    }),
    inlayHints: (): HintSettings => ({
        groups: read("inlayHints.groups", false),
        matches: read("inlayHints.matches", true)
    }),
    exploreDepth: (): number => read("explore.depth", 4)
};

function read<T>(name: string, fallback: T): T {
    return workspace.getConfiguration(SECTION).get<T>(name, fallback);
}
