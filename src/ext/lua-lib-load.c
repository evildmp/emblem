#include "lua-lib-load.h"

#include "ext-params.h"
#include "logs/logs.h"
#include <lauxlib.h>

int load_em_std_lib(ExtensionState* s)
{
	int rc = 0;
#include "lib/std/std.lc"
#include "lib/std/lingo.lc"
	return rc;
}
