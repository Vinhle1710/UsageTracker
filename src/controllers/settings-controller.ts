import type { Config } from "../types";
export class SettingsController {
  constructor(private readonly onChange: (config: Config) => void) {}
  update(config: Config): void { this.onChange(config); }
}
