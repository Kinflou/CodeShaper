import { MarkdownString, TextDocument, Uri } from "vscode";
import { settings } from "../../settings";
import { COMMANDS, LINKED } from "../commands";
import { editorOf } from "../copy";
import { Block, BlockName, BlockPath, BlockUse } from "../protocol";
import { live } from "../ranges";
import { State } from "../state";
import { closed, code, commandLink, escape } from "./markdown";
import { formatted } from "./query";
import { sample } from "./sample";

const QUERY_LANGUAGE = "codeshaper-query";
const CODE_PADDING = "&thinsp;&thinsp;";
const BETWEEN = " · ";
const SHOWN_USES = 10;

type Link = (title: string, command: string, path?: number) => string;

export function popup(state: State, document: TextDocument, block: Block): MarkdownString {
    const id = state.remember(document, block);
    const link: Link = (title, command, path) => commandLink(title, command, id, path);
    const text = new MarkdownString();

    text.isTrusted = { enabledCommands: LINKED };
    text.supportThemeIcons = true;
    addActions(text, document, block, link);
    addBlock(text, document, block);
    addLocations(text, block.paths, link);
    addQuery(text, block, link);
    addUses(text, block.used);

    return text;
}

function addActions(text: MarkdownString, document: TextDocument, block: Block, link: Link): void {
    const isSelected = editorOf(document)?.selection.contains(live(block.range)) ?? false;
    const actions = [
        ...(isSelected ? [] : [link("Highlight", COMMANDS.highlight)]),
        ...(block.parents.length > 0 ? [link("Parent", COMMANDS.parent)] : [])
    ];

    if (actions.length > 0) {
        text.appendMarkdown(`${actions.join(BETWEEN)}\n\n`);
    }
}

function addBlock(text: MarkdownString, document: TextDocument, block: Block): void {
    const parents = block.parents.map(title).reverse();
    const inside = parents.length > 0 ? ` in ${parents.join(" › ")}` : "";
    const range = live(block.range);
    const shown = sample(document.getText(range), document.lineAt(range.start.line).text, settings.hover());

    text.appendMarkdown(`**${title(block)}**${inside}\n\n`);
    text.appendCodeblock(shown, document.languageId);
    text.appendMarkdown("\n");
    if (block.broken) {
        text.appendMarkdown("Near a syntax error, locations can be wrong here\n\n");
    }
}

function addLocations(text: MarkdownString, paths: BlockPath[], link: Link): void {
    if (paths.length > 0) {
        text.appendMarkdown("### Locations:\n\n");
    }

    for (const [path, { location, places, number }] of paths.entries()) {
        const actions = [
            link("Highlight", COMMANDS.highlight, path),
            link("Next", COMMANDS.next, path),
            link("Previous", COMMANDS.previous, path),
            link("All places", COMMANDS.all, path),
            link("Copy", COMMANDS.copy, path)
        ];
        text.appendMarkdown(`- ${code(location)} - place ${number ?? "?"} of ${places} in this file  \n  ${CODE_PADDING}${actions.join(BETWEEN)}\n`);
    }
}

function addQuery(text: MarkdownString, { query, queryLanguage, paths }: Block, link: Link): void {
    if (!query) {
        return;
    }

    const kind = queryLanguage ? `[${escape(queryLanguage.name)} query](${closed(queryLanguage.documentation)})` : "query";
    const button = link("$(copy) Copy", COMMANDS.copy, paths.length);

    text.appendMarkdown(`\n\n### Exact location (${kind}):\n\nMatches only this block in this file${BETWEEN}${button}\n`);
    text.appendCodeblock(formatted(query), QUERY_LANGUAGE);
}

function addUses(text: MarkdownString, used: BlockUse[]): void {
    if (used.length === 0) {
        return;
    }

    text.appendMarkdown("\n\n### Used in:\n\n");
    for (const use of used.slice(0, SHOWN_USES)) {
        const { line, character } = use.range.start;
        const uri = Uri.parse(use.uri).with({ fragment: `L${line + 1},${character + 1}` });
        const name = `[${escape(`${use.patch} › ${use.action}`)}](${closed(uri.toString())})`;
        const notes = [use.field === "location" ? "" : escape(use.field), use.enabled ? "" : "disabled", use.whole ? "" : "around it"];
        const noted = notes.filter(note => note !== "").join(", ");

        text.appendMarkdown(`- ${name}${noted ? ` (${noted})` : ""}\n`);
    }
    if (used.length > SHOWN_USES) {
        text.appendMarkdown(`- and ${used.length - SHOWN_USES} more\n`);
    }
}

function title({ kind, field }: BlockName): string {
    return escape(field ?? kind);
}
