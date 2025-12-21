package dev.waylon.terminal.boundedcontexts.terminalsession.application.useCase

import dev.waylon.terminal.boundedcontexts.terminalsession.application.service.TerminalSessionService
import dev.waylon.terminal.boundedcontexts.terminalsession.domain.TerminalSession
import org.slf4j.LoggerFactory

/**
 * Get terminal session by ID use case
 * Encapsulates the business logic for getting a terminal session by ID, keeping the Route layer lightweight
 */
class GetTerminalSessionByIdUseCase(
    private val terminalSessionService: TerminalSessionService
) {
    private val log = LoggerFactory.getLogger(GetTerminalSessionByIdUseCase::class.java)

    /**
     * Execute the operation to get a terminal session by ID
     * @param sessionId The session ID
     * @return The terminal session, or null if it doesn't exist
     */
    fun execute(sessionId: String): TerminalSession? {
        log.debug("Executing GetTerminalSessionByIdUseCase for sessionId: {}", sessionId)
        
        val session = terminalSessionService.getSessionById(sessionId)
        
        if (session != null) {
            log.debug("Found session: {}, status: {}", sessionId, session.status)
        } else {
            log.debug("Session not found: {}", sessionId)
        }
        
        return session
    }
}
