package dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase

import dev.waylon.terminal.boundedcontexts.terminalsession.application.service.TerminalSessionService
import dev.waylon.terminal.boundedcontexts.terminalsession.domain.TerminalSession
import org.slf4j.LoggerFactory

/**
 * Terminate terminal session use case
 * Encapsulates the business logic for terminating a terminal session, keeping the Route layer lightweight
 */
class TerminateTerminalSessionUseCase(
    private val terminalSessionService: TerminalSessionService
) {
    private val log = LoggerFactory.getLogger(TerminateTerminalSessionUseCase::class.java)

    /**
     * Execute the operation to terminate a terminal session
     * @param sessionId The session ID
     * @return The terminated terminal session, or null if it doesn't exist
     */
    fun execute(sessionId: String): TerminalSession? {
        log.debug("Executing TerminateTerminalSessionUseCase for sessionId: {}", sessionId)
        
        val session = terminalSessionService.terminateSession(sessionId)
        
        if (session != null) {
            log.info("Terminated terminal session: {}", sessionId)
        } else {
            log.debug("Failed to terminate terminal session: {}", sessionId)
        }
        
        return session
    }
}
