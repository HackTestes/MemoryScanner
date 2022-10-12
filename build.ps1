# PowerShell version 7.2.6

cd ".\src\" &&

#($SOURCE_FILES = Get-ChildItem "*.cpp") &&

Get-ChildItem "*.cpp" | Foreach-Object {

    $Job = Start-Job -ScriptBlock {cl.exe "$($_.BaseName).cpp" /c "$($_.BaseName).obj"}

}

echo "Compilation done" &&

($Obj_files = Get-ChildItem -Name "*.obj") &&

cl.exe $Obj_files /Fe:MemoryScanner.exe;

cd "..";