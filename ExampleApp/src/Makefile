TOP=../..

include $(TOP)/configure/CONFIG
#----------------------------------------
#  ADD MACRO DEFINITIONS BELOW HERE

# Build the IOC application
PROD_IOC = Example

# Example.dbd will be created and installed
DBD += Example.dbd

# Example.dbd will include these files:
Example_DBD += base.dbd

# Example_registerRecordDeviceDriver.cpp derives from Example.dbd
Example_SRCS += Example_registerRecordDeviceDriver.cpp

# Build the main IOC entry point where needed
Example_SRCS += ExampleMain.cpp

# Use Ferrite
Example_DBD += Ferrite.dbd
Example_LIBS += Ferrite
SHRLIB_SEARCH_DIRS += $(TOP)/target/debug
Example_LIBS += backend # -Wl,-Bdynamic -l:libbackend.so

# Finally link IOC to the EPICS Base libraries
Example_LIBS += $(EPICS_BASE_IOC_LIBS)

include $(TOP)/configure/RULES
#----------------------------------------
#  ADD EXTRA GNUMAKE RULES BELOW HERE
