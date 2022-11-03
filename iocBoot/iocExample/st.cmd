#!../../bin/linux-x86_64/Example

#- You may have to change Example to something else
#- everywhere it appears in this file

< envPaths

cd "${TOP}"

## Register all support components
dbLoadDatabase "dbd/Example.dbd"
Example_registerRecordDeviceDriver pdbbase

## Load record instances
dbLoadRecords "db/Example.db"

cd "${TOP}/iocBoot/${IOC}"
iocInit
