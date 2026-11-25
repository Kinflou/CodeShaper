const KEY = "codeshaper.variables";

export type Values = Record<string, string>;

export interface Memory {
    get<T>(key: string): T | undefined;
    update(key: string, value: unknown): Thenable<void>;
}

export class ValueStore {
    constructor(private readonly memory: Memory) {}

    of(project: string): Values {
        return { ...this.all()[project] };
    }

    async set(project: string, name: string, value: string): Promise<void> {
        await this.replace(project, { ...this.of(project), [name]: value });
    }

    async reset(project: string, name: string): Promise<void> {
        const { [name]: _removed, ...rest } = this.of(project);

        await this.replace(project, rest);
    }

    private async replace(project: string, values: Values): Promise<void> {
        const { [project]: _replaced, ...others } = this.all();
        const all = Object.keys(values).length === 0 ? others : { ...others, [project]: values };

        await this.memory.update(KEY, all);
    }

    private all(): Record<string, Values> {
        return this.memory.get<Record<string, Values>>(KEY) ?? {};
    }
}
