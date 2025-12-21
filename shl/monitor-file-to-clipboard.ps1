param(
    [Parameter(Mandatory=$true)]
    [string]$FilePath,
    
    [int]$CheckInterval = 1000
)

Write-Host "监控文件: $FilePath"
Write-Host "检查间隔: $CheckInterval ms"
Write-Host "按 Ctrl+C 停止监控..."
Write-Host ""

# 初始化上一次文件大小
$lastSize = 0

while ($true) {
    try {
        if (Test-Path -Path $FilePath -PathType Leaf) {
            $fileInfo = Get-Item -Path $FilePath
            $currentSize = $fileInfo.Length
            
            # 只有当文件大小大于0且与上次不同时才复制
            if ($currentSize -gt 0 -and $currentSize -ne $lastSize) {
                Write-Host "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] 文件有新内容，复制到剪贴板..."
                $content = Get-Content -Path $FilePath -Raw
                Set-Clipboard -Value $content
                Write-Host "内容已复制到剪贴板"
                $lastSize = $currentSize
            }
        } else {
            Write-Host "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] 文件不存在: $FilePath"
        }
    } catch {
        Write-Host "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] 错误: $($_.Exception.Message)"
    }
    
    # 等待指定间隔
    Start-Sleep -Milliseconds $CheckInterval
}