use super::*;
use crate::model::ServerSelection;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerPage {
    data: Vec<PublicServer>,
    next_page_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicServer {
    id: String,
    playing: u32,
    max_players: u32,
    #[serde(default)]
    ping: Option<f64>,
}

impl PublicServer {
    fn ping(&self) -> Option<f64> {
        self.ping.filter(|p| p.is_finite() && *p > 0.0)
    }
}

fn choose_server(servers: &[PublicServer], mode: ServerSelection, seats: u32) -> Option<Uuid> {
    let candidates: Vec<_> = servers
        .iter()
        .filter_map(|s| {
            let id = Uuid::parse_str(&s.id).ok().filter(|id| !id.is_nil())?;
            (s.max_players.saturating_sub(s.playing) >= seats.max(1)).then_some((s, id))
        })
        .collect();
    // Reported ping is a server statistic, not a local latency or region measurement.
    // Prefer the least populated server within 25ms of the best reported ping.
    let best_ping = candidates
        .iter()
        .filter_map(|(s, _)| s.ping())
        .reduce(f64::min);
    candidates
        .into_iter()
        .filter(|(s, _)| {
            mode != ServerSelection::Smart
                || best_ping.is_none_or(|best| s.ping().is_some_and(|p| p <= best + 25.0))
        })
        .min_by(|(a, aid), (b, bid)| {
            let population = if mode == ServerSelection::Highest {
                b.playing.cmp(&a.playing)
            } else {
                a.playing.cmp(&b.playing)
            };
            population
                .then_with(|| {
                    a.ping()
                        .unwrap_or(f64::MAX)
                        .total_cmp(&b.ping().unwrap_or(f64::MAX))
                })
                .then_with(|| aid.cmp(bid))
        })
        .map(|(_, id)| id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PresencePage {
    user_presences: Vec<Presence>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Presence {
    user_id: u64,
    user_presence_type: u8,
    place_id: Option<u64>,
    game_id: Option<String>,
}
fn friend_job(presences: &[Presence], users: &[u64], place: u64) -> Option<Uuid> {
    users.iter().find_map(|user| {
        presences
            .iter()
            .find(|p| p.user_id == *user)
            .filter(|p| p.user_presence_type == 2 && p.place_id == Some(place))
            .and_then(|p| p.game_id.as_deref())
            .and_then(|id| Uuid::parse_str(id).ok())
            .filter(|id| !id.is_nil())
    })
}

impl Roblox {
    pub async fn select_public_server(
        &self,
        cookie: &str,
        place: u64,
        mode: ServerSelection,
        seats: u32,
    ) -> Result<Option<Uuid>, Failure> {
        let header = Self::cookie_header(cookie)?;
        let mut cursor = None;
        let mut seen = std::collections::HashSet::new();
        let mut servers = Vec::new();
        // ponytail: inspect at most 500 servers per launch; avoid unbounded scans of large games.
        for _ in 0..5 {
            let mut request = self
                .client
                .get(format!(
                    "https://games.roblox.com/v1/games/{place}/servers/Public"
                ))
                .header(COOKIE, header.clone())
                .query(&[
                    (
                        "sortOrder",
                        if mode == ServerSelection::Highest {
                            "Desc"
                        } else {
                            "Asc"
                        },
                    ),
                    ("limit", "100"),
                    ("excludeFullGames", "true"),
                ]);
            if let Some(value) = &cursor {
                request = request.query(&[("cursor", value)]);
            }
            let response = request.send().await.map_err(|_| Failure::network())?;
            Self::check(response.status(), "Public server selection")?;
            let page: ServerPage = response.json().await.map_err(|_| {
                Failure::new(
                    FailureKind::Unsupported,
                    "Roblox public server response was not recognized",
                )
            })?;
            servers.extend(page.data);
            if mode != ServerSelection::Smart
                && let Some(id) = choose_server(&servers, mode, seats)
            {
                return Ok(Some(id));
            }
            cursor = page.next_page_cursor.filter(|c| !c.is_empty());
            if cursor.as_ref().is_none_or(|c| !seen.insert(c.clone())) {
                break;
            }
        }
        if servers.is_empty() {
            return Ok(None);
        } // Let Roblox start a server for an empty game.
        choose_server(&servers, mode, seats).map(Some).ok_or_else(|| Failure::new(FailureKind::Other, "No listed public server has enough open slots; retry later or choose another destination"))
    }

    pub async fn friend_server(
        &self,
        cookie: &str,
        users: &[u64],
        place: u64,
    ) -> Result<Option<Uuid>, Failure> {
        let url = "https://presence.roblox.com/v1/presence/users";
        #[cfg(test)]
        let url = self.presence_url.as_deref().unwrap_or(url);
        let request = self
            .client
            .post(url)
            .header(COOKIE, Self::cookie_header(cookie)?)
            .json(&serde_json::json!({"userIds": users}));
        let first = request
            .try_clone()
            .ok_or_else(|| Failure::new(FailureKind::Other, "Unable to prepare friend lookup"))?
            .send()
            .await
            .map_err(|_| Failure::network())?;
        let response = if first.status() == StatusCode::FORBIDDEN
            && let Some(csrf) = first.headers().get("x-csrf-token").cloned()
        {
            request
                .header("x-csrf-token", csrf)
                .send()
                .await
                .map_err(|_| Failure::network())?
        } else {
            first
        };
        Self::check(response.status(), "Same-preset friend lookup")?;
        let page: PresencePage = response.json().await.map_err(|_| {
            Failure::new(
                FailureKind::Unsupported,
                "Roblox presence response was not recognized",
            )
        })?;
        Ok(friend_job(&page.user_presences, users, place))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranking_and_friend_presence_respect_capacity_missing_ping_and_place() {
        let data = serde_json::json!({"data": [
            {"id":"00000000-0000-0000-0000-000000000001","playing":1,"maxPlayers":10,"ping":250},
            {"id":"00000000-0000-0000-0000-000000000002","playing":3,"maxPlayers":10,"ping":60},
            {"id":"00000000-0000-0000-0000-000000000003","playing":7,"maxPlayers":10,"ping":40},
            {"id":"00000000-0000-0000-0000-000000000004","playing":0,"maxPlayers":10,"ping":0},
            {"id":"00000000-0000-0000-0000-000000000005","playing":10,"maxPlayers":10,"ping":10},
            {"id":"invalid","playing":0,"maxPlayers":10,"ping":1}
        ], "nextPageCursor":null});
        let page: ServerPage = serde_json::from_value(data).unwrap();
        assert_eq!(
            choose_server(&page.data, ServerSelection::Smart, 1)
                .unwrap()
                .as_u128(),
            2
        );
        assert_eq!(
            choose_server(&page.data, ServerSelection::Lowest, 1)
                .unwrap()
                .as_u128(),
            4
        );
        assert_eq!(
            choose_server(&page.data, ServerSelection::Highest, 1)
                .unwrap()
                .as_u128(),
            3
        );
        assert_eq!(
            choose_server(&page.data, ServerSelection::Highest, 5)
                .unwrap()
                .as_u128(),
            2
        );
        assert!(choose_server(&page.data, ServerSelection::Smart, 11).is_none());
        let unknown: ServerPage = serde_json::from_value(serde_json::json!({"data":[{"id":"00000000-0000-0000-0000-000000000001","playing":0,"maxPlayers":2}],"nextPageCursor":null})).unwrap();
        assert!(choose_server(&unknown.data, ServerSelection::Smart, 1).is_some());
        let presence: PresencePage = serde_json::from_value(serde_json::json!({"userPresences":[
            {"userId":1,"userPresenceType":2,"placeId":99,"gameId":"00000000-0000-0000-0000-000000000003"},
            {"userId":2,"userPresenceType":2,"placeId":10,"gameId":"00000000-0000-0000-0000-000000000002"},
            {"userId":3,"userPresenceType":2,"placeId":null,"gameId":null}
        ]})).unwrap();
        assert_eq!(
            friend_job(&presence.user_presences, &[1, 2, 3], 10)
                .unwrap()
                .as_u128(),
            2
        );
        assert!(friend_job(&presence.user_presences, &[1, 3], 10).is_none());
    }
}
