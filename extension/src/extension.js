const path = require("path");
const fs = require("fs");
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");
let client;
let terminal;
export function activate(context) {
  const jsonpilerPath = path.join(
    context.extensionPath,
    "bin",
    "jsonpiler.exe",
  );
  if (!fs.existsSync(jsonpilerPath)) {
    vscode.window.showErrorMessage("jsonpiler.exe not found");
    return;
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
    terminal.sendText(isWin ? "cls" : "clear", true);
    terminal.show(true);
    const shell = vscode.env.shell;
    const isPowerShell = shell.toLowerCase().includes("powershell");
    const cmd = isPowerShell
      ? `& "${jsonpilerPath}" "${file}"`
      : `"${jsonpilerPath}" "${file}"`;
    terminal.sendText(cmd, true);
  });
  context.subscriptions.push(disposable);
  const serverOptions = {
    command: jsonpilerPath,
    args: ["server"],
    transport: TransportKind.stdio,
  };
  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "jspl" }],
  };
  client = new LanguageClient(
    "jsonpiler",
    "Jsonpiler LSP",
    serverOptions,
    clientOptions,
  );
  context.subscriptions.push(client.start());
}
export function deactivate() {
  terminal?.dispose();
  return client?.stop();
}
