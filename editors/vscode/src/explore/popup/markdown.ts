export function code(text: string): string {
    return `\`${text.replace(/`/g, "'")}\``;
}

export function closed(link: string): string {
    return link.replace(/[()]/g, found => found === "(" ? "%28" : "%29");
}

export function escape(text: string): string {
    return text.replace(/[\\`*_{}[\]()#+\-.!|<>~&$]/g, "\\$&");
}

export function commandLink(title: string, command: string, popup: number, path?: number): string {
    const argument = path === undefined ? `%5B${popup}%5D` : `%5B${popup}%2C${path}%5D`;

    return `[${title}](command:${command}?${argument})`;
}
