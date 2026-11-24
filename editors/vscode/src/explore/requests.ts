import { window, CancellationToken } from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { reason } from "../errors";
import { Subject } from "./copy";
import { Block, BlockHint, FilesResult, Place, PlainLocation, PlainRange } from "./protocol";

export class Requests {
    constructor(private readonly client: LanguageClient) {}

    files(uri: string): Promise<FilesResult | undefined> {
        return this.loud("codeshaper/explore", { uri });
    }

    block(subject: Subject, range: PlainRange): Promise<Block | null | undefined> {
        return this.loud("codeshaper/exploreBlock", { ...about(subject), range });
    }

    glimpse(subject: Subject, range: PlainRange, token: CancellationToken, node?: number): Promise<Block | null | undefined> {
        return this.quiet("codeshaper/exploreBlock", { ...about(subject), range, node }, token);
    }

    hints(subject: Subject, depth: number, all: boolean, token: CancellationToken): Promise<BlockHint[] | undefined> {
        return this.quiet("codeshaper/exploreHints", { ...about(subject), depth, all }, token);
    }

    place(subject: Subject, range: PlainRange, location: string, forward: boolean): Promise<Place | null | undefined> {
        return this.loud("codeshaper/explorePlace", { ...about(subject), range, location, forward });
    }

    places(subject: Subject, location: string): Promise<PlainLocation[] | undefined> {
        return this.loud("codeshaper/explorePlaces", { ...about(subject), location });
    }

    private async quiet<T>(method: string, params: object, token: CancellationToken): Promise<T | undefined> {
        try {
            return await this.client.sendRequest<T>(method, params, token);
        } catch {
            return undefined;
        }
    }

    private async loud<T>(method: string, params: object): Promise<T | undefined> {
        try {
            return await this.client.sendRequest<T>(method, params);
        } catch (error) {
            window.showErrorMessage(`CodeShaper could not explore: ${reason(error)}`);
            return undefined;
        }
    }
}

function about({ document, start, file }: Subject): object {
    return { uri: start, file, text: document.getText() };
}
