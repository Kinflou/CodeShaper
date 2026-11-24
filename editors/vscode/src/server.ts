import * as fs from "fs";
import * as path from "path";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import { settings } from "./settings";

export function createClient(extensionPath: string): LanguageClient {
    const serverOptions: ServerOptions = { command: executable(settings.serverPath(), extensionPath), args: ["lsp"] };
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "kdl" }],
        initializationOptions: {
            suggestAgain: true,
            hover: settings.hover(),
            inlayHints: settings.inlayHints()
        }
    };

    return new LanguageClient("codeshaper", "CodeShaper", serverOptions, clientOptions);
}

function executable(configured: string, extensionPath: string): string {
    const name = process.platform === "win32" ? "codeshaper.exe" : "codeshaper";
    const bundled = path.join(extensionPath, "server", name);

    return configured.trim() || (fs.existsSync(bundled) ? bundled : "codeshaper");
}
