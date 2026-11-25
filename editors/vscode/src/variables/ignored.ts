import { execFile } from "child_process";
import * as fs from "fs";
import * as path from "path";

export async function ignored(paths: string[]): Promise<Set<string>> {
    const repositories = new Map<string, string[]>();
    for (const file of paths) {
        const root = repository(path.dirname(file));
        if (root !== undefined) {
            repositories.set(root, [...repositories.get(root) ?? [], file]);
        }
    }
    if (repositories.size === 0) {
        return new Set();
    }

    const outside = await ignored([...repositories.keys()]);
    const found = await Promise.all([...repositories].map(([root, files]) => outside.has(root) ? files : ask(root, files)));

    return new Set(found.flat());
}

function repository(directory: string): string | undefined {
    if (fs.existsSync(path.join(directory, ".git"))) {
        return directory;
    }
    const parent = path.dirname(directory);

    return parent === directory ? undefined : repository(parent);
}

function ask(root: string, files: string[]): Promise<string[]> {
    return new Promise(resolve => {
        const git = execFile("git", ["-C", root, "check-ignore", "-z", "--stdin"], { maxBuffer: Infinity }, (_error, output) => {
            const answer = new Set(output.split("\0"));
            resolve(files.filter(file => answer.has(file)));
        });
        git.stdin?.on("error", () => {});
        git.stdin?.end(files.join("\0"));
    });
}
