import SwiftUI

struct SiteStatusPresentation {
  let title: String
  let symbol: String
  let color: Color

  init(service: ServiceEntry, action: ServiceAction? = nil) {
    switch action {
    case .start:
      title = "Starting…"
      symbol = "arrow.clockwise"
      color = .blue
      return
    case .stop:
      title = "Stopping…"
      symbol = "stop.circle"
      color = .blue
      return
    case .restart:
      title = "Restarting…"
      symbol = "arrow.clockwise"
      color = .blue
      return
    case .open, nil:
      break
    }
    title = service.stateLabel
    if service.error != nil || service.report == nil {
      symbol = "questionmark.circle"
      color = .orange
    } else if service.report?.status.phase == "failed" {
      symbol = "exclamationmark.circle.fill"
      color = .red
    } else if service.report?.status.phase == "unhealthy"
      || service.report?.status.phase == "unknown"
      || (service.report?.status.processAlive == true
        && service.report?.status.identityMatches != true)
      || service.report?.missingFields.isEmpty == false
    {
      symbol = "exclamationmark.triangle.fill"
      color = .orange
    } else if service.report?.status.phase == "starting" {
      symbol = "clock"
      color = .blue
    } else if service.report?.status.phase == "ready"
      || service.report?.status.readinessPassed == true
    {
      symbol = "checkmark.circle.fill"
      color = .green
    } else {
      symbol = "circle"
      color = .secondary
    }
  }
}
