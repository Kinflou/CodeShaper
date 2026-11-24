import { Hover, Location, LocationLink, MarkdownString, Position, TextDocument, Uri } from "vscode";
import { Middleware } from "vscode-languageclient/node";
import { COMMANDS } from "./commands";
import { copyUri } from "./copy";
import { Asked, Links } from "./links";
import { Requests } from "./requests";

export function middleware(links: Links, requests: () => Requests): Middleware {
    const copied = async <T extends Location | LocationLink>(found: T[], document: TextDocument, position: Position): Promise<T[]> => {
        if (found.every(result => place(result).toString() === document.uri.toString())) {
            return found;
        }

        const asked = about(document, position);
        const explored = await requests().from(asked);
        const copies = new Map(explored?.files.map(file => [
            Uri.parse(file.uri).fsPath, copyUri({ start: asked.uri, patch: explored.patch ?? undefined }, file)
        ]));

        return found.map(result => moved(result, copies.get(place(result).fsPath)));
    };

    return {
        provideHover: async (document, position, token, next) => {
            const hover = await next(document, position, token);

            return hover && new Hover(hover.contents.map(content => linked(content, links, about(document, position))), hover.range);
        },
        provideReferences: async (document, position, options, token, next) => {
            const found = await next(document, position, options, token);

            return found && copied(found, document, position);
        },
        provideDefinition: async (document, position, token, next) => {
            const found = await next(document, position, token);
            const results = found && (Array.isArray(found) ? found : [found]) as LocationLink[];

            return results && copied(results, document, position);
        }
    };
}

function about(document: TextDocument, { line, character }: Position): Asked {
    return { uri: document.uri.toString(), position: { line, character } };
}

function linked(content: Hover["contents"][number], links: Links, asked: Asked): Hover["contents"][number] {
    if (!(content instanceof MarkdownString)) {
        return content;
    }

    const text = new MarkdownString(links.replaced(content.value, COMMANDS.open, asked));
    text.isTrusted = { enabledCommands: [COMMANDS.open] };

    return text;
}

function place(result: Location | LocationLink): Uri {
    return "targetUri" in result ? result.targetUri : result.uri;
}

function moved<T extends Location | LocationLink>(result: T, copy?: Uri): T {
    if (!copy) {
        return result;
    }

    return "targetUri" in result ? { ...result, targetUri: copy } : new Location(copy, result.range) as T;
}
