import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { LanguageClient, TransportKind } from "vscode-languageclient/node";
let client: LanguageClient | undefined;
let terminal: vscode.Terminal | undefined;
export function activate(context: vscode.ExtensionContext): void {
  const jsonpilerName =
    process.platform === "win32" ? "jsonpiler.exe" : "jsonpiler";
  const jsonpilerPath = path.join(context.extensionPath, "bin", jsonpilerName);
  if (!fs.existsSync(jsonpilerPath)) {
    vscode.window.showErrorMessage("jsonpiler not found");
    return;
  }
  if (process.platform === "darwin" || process.platform === "linux") {
    try {
      fs.chmodSync(jsonpilerPath, 0o755);
    } catch (e) {
      console.error("chmod failed:", e);
    }
  }
  const disposable = vscode.commands.registerCommand("jspl.run", async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor) return;
    const doc = editor.document;
    if (doc.isUntitled) {
      vscode.window.showErrorMessage("Please save file.");
      return;
    }
    if (doc.isDirty) {
      try {
        await doc.save();
      } catch {
        vscode.window.showErrorMessage("Failed to save file.");
        return;
      }
    }
    const file = doc.fileName;
    if (!file) return;
    if (!terminal || terminal.exitStatus) {
      terminal = vscode.window.createTerminal("JSPL");
    }
    const isWin = process.platform === "win32";
    const shell = vscode.env.shell;
    const isPowerShell =
      shell.toLowerCase().includes("powershell") || shell.includes("pwsh");
    if (isPowerShell) {
      terminal.sendText(
        "Set-PSReadLineOption -HistorySaveStyle SaveNothing",
        true,
      );
    }
    terminal.sendText(isWin ? "cls" : "clear", true);
    terminal.show(true);
    const cmd = isPowerShell
      ? `& "${jsonpilerPath}" "${file}"`
      : `"${jsonpilerPath}" "${file}"`;
    terminal.sendText(cmd, true);
  });
  context.subscriptions.push(disposable);
  const outputChannel = vscode.window.createOutputChannel(
    "JSPL Language Server Trace",
    { log: true },
  );
  const traceOutputChannel = vscode.window.createOutputChannel(
    "JSPL Client Trace",
    { log: true },
  );
  const serverOptions = {
    command: jsonpilerPath,
    args: ["server"],
    transport: TransportKind.stdio,
  };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "jspl" }],
    outputChannel,
    traceOutputChannel,
  };
  client = new LanguageClient(
    "jspl",
    "JSPL Language Server",
    serverOptions,
    clientOptions,
  );
  context.subscriptions.push(client);
  client.start();
}
export function deactivate(): Thenable<void> | undefined {
  terminal?.dispose();
  return client?.stop();
}
