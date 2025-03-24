$ErrorActionPreference = "Stop"

try {
  Write-Host "Pushing to jsompiler.git..."
  git remote set-url origin https://github.com/HAL-G1THuB/jsompiler.git
  git add .
    git commit -m "Update main repository"
    git push origin main
    Write-Host "Successfully pushed to jsompiler.git."
} catch {
  Write-Host "An error occurred: $_" -ForegroundColor Red
  exit 1
}
