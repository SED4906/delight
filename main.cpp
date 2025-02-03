#include <sys/wait.h>
#include <signal.h>
#include <unistd.h>

#include "unit.h"

#define TIMEOUT 30

static void reap(void);

static sigset_t set;

int main(void) {
  if (getpid() != 1)
    return 1;
  chdir("/");
  sigfillset(&set);
  sigprocmask(SIG_BLOCK, &set, NULL);
  load_unit("default.target");
  for (int sig;;) {
    alarm(TIMEOUT);
    sigwait(&set, &sig);
    switch (sig) {
    case SIGCHLD:
    case SIGALRM:
      reap();
    default:
      break;
    }
  }
  return 0;
}

static void reap(void) {
  while (waitpid(-1, NULL, WNOHANG) > 0)
    ;
  alarm(TIMEOUT);
}
