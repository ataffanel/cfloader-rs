// Crazyflie bootloader link implementation
// The bootloader link is very similar to the Crazylfie link over ESB except that it is
// based on a very early iteration and does not implement safelink
// We will be using is as a half-duplex link in this case, only sending or receiving at a time

use crazyradio::{Crazyradio, SharedCrazyradio};
use std::time::Duration;

pub struct Bllink {
    radio: SharedCrazyradio,
    address: [u8; 5],
    channel: crazyradio::Channel,
}

const DEFAULT_ADDRESS: [u8; 5] = [0xE7, 0xE7, 0xE7, 0xE7, 0xE7];
const BOOTLOADER_CHANNEL: u8 = 0; // Bootloader channel
const MAX_RETRIES: usize = 10; // Maximum number of retries for packet transmission

impl Bllink {
    pub async fn new(address: Option<&[u8; 5]>) -> anyhow::Result<Self> {
        let address = address.unwrap_or(&DEFAULT_ADDRESS);

        let radio = Crazyradio::open_first_async().await?;
        let radio = SharedCrazyradio::new(radio);

        // TODO: Check connectivity by sending a ping or similar

        Ok(Bllink { radio, channel: crazyradio::Channel::from_number(BOOTLOADER_CHANNEL).unwrap(), address: *address })
    }


    // Send a packet as request, expect one packet as response
    pub async fn request(&mut self, data: &[u8], timeout_duration: Duration) -> anyhow::Result<Vec<u8>> {
        for attempt in 0..MAX_RETRIES {
            match self.try_request(data, timeout_duration).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if attempt == MAX_RETRIES - 1 {
                        return Err(anyhow::anyhow!(
                            "Failed to get response after {} attempts: {}", 
                            MAX_RETRIES, e
                        ));
                    }
                    // Log retry attempt if desired
                    //eprintln!("Request attempt {} failed: {}, retrying...", attempt + 1, e);
                }
            }
        }
        unreachable!()
    }

    // Send a packet as request, expect one packet as response. The first n bytes of the response must match the request
    pub async fn request_match_response(&mut self, data: &[u8], match_length: usize, timeout_duration: Duration) -> anyhow::Result<Vec<u8>> {
        for attempt in 0..MAX_RETRIES {
            match self.try_request_match_response(data, match_length, timeout_duration).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if attempt == MAX_RETRIES - 1 {
                        return Err(anyhow::anyhow!(
                            "Failed to get matching response after {} attempts: {}", 
                            MAX_RETRIES, e
                        ));
                    }
                    // Log retry attempt if desired
                    //eprintln!("Request match attempt {} failed: {}, retrying...", attempt + 1, e);
                }
            }
        }
        unreachable!()
    }

    // Internal method to try a single request with partial response matching
    async fn try_request_match_response(&mut self, data: &[u8], match_length: usize, timeout_duration: Duration) -> anyhow::Result<Vec<u8>> {
        let start_time = std::time::Instant::now();
        let mut answer = Vec::new();
        let mut got_initial_ack = false;
        
        // Validate match_length
        if match_length > data.len() {
            return Err(anyhow::anyhow!("match_length {} cannot be greater than data length {}", match_length, data.len()));
        }
        
        let match_data = &data[..match_length];
        
        // First, send the initial request and wait for ACK within timeout window
        while start_time.elapsed() < timeout_duration && !got_initial_ack {
            let (ack, response) = self.radio.send_packet_async(self.channel, self.address, data.to_vec()).await
                .map_err(|e| anyhow::anyhow!("Radio error during initial send: {}", e))?;

            if ack.received {
                got_initial_ack = true;
                answer = response;
            } else {
                // Short delay before retry
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }
        
        if !got_initial_ack {
            return Err(anyhow::anyhow!("Timeout: No ACK received for initial packet within {:?}", timeout_duration));
        }

        // Keep polling for valid response with remaining timeout
        while start_time.elapsed() < timeout_duration && (answer.len() < match_length || !answer[..match_length].eq(match_data)) {
            let (new_ack, new_answer) = self.radio.send_packet_async(self.channel, self.address, vec![0xff]).await
                .map_err(|e| anyhow::anyhow!("Radio error during polling: {}", e))?;

            if new_ack.received {
                answer = new_answer;
            }
            
            // Short delay before next poll
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        
        if answer.len() < match_length || !answer[..match_length].eq(match_data) {
            return Err(anyhow::anyhow!(
                "Timeout: No valid response received within {:?}. Expected first {} bytes to match {:02X?}, got {:02X?}", 
                timeout_duration, match_length, match_data, 
                if answer.len() >= match_length { &answer[..match_length] } else { &answer }
            ));
        }

        Ok(answer)
    }

    // Internal method to try a single request with timeout
    async fn try_request(&mut self, data: &[u8], timeout_duration: Duration) -> anyhow::Result<Vec<u8>> {
        let start_time = std::time::Instant::now();
        let mut answer = Vec::new();
        let mut got_initial_ack = false;
        
        // First, send the initial request and wait for ACK within timeout window
        while start_time.elapsed() < timeout_duration && !got_initial_ack {
            let (ack, response) = self.radio.send_packet_async(self.channel, self.address, data.to_vec()).await
                .map_err(|e| anyhow::anyhow!("Radio error during initial send: {}", e))?;

            if ack.received {
                got_initial_ack = true;
                answer = response;
            } else {
                // Short delay before retry
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }
        
        if !got_initial_ack {
            return Err(anyhow::anyhow!("Timeout: No ACK received for initial packet within {:?}", timeout_duration));
        }

        // Keep polling for valid response with remaining timeout
        while start_time.elapsed() < timeout_duration && !answer.starts_with(data) {
            let (new_ack, new_answer) = self.radio.send_packet_async(self.channel, self.address, vec![0xff]).await
                .map_err(|e| anyhow::anyhow!("Radio error during polling: {}", e))?;

            if new_ack.received {
                answer = new_answer;
            }
            
            // Short delay before next poll
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        
        if !answer.starts_with(data) {
            return Err(anyhow::anyhow!("Timeout: No valid response received within {:?}", timeout_duration));
        }

        Ok(answer)
    }

    // Send a packet as request, expect no response
    pub async fn send(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.send_with_timeout(data, Duration::from_millis(1000)).await
    }

    // Send a packet with timeout and retry logic, expect no response
    pub async fn send_with_timeout(&mut self, data: &[u8], timeout_duration: Duration) -> anyhow::Result<()> {
        for attempt in 0..MAX_RETRIES {
            match self.try_send(data, timeout_duration).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    if attempt == MAX_RETRIES - 1 {
                        return Err(anyhow::anyhow!(
                            "Failed to send packet after {} attempts: {}", 
                            MAX_RETRIES, e
                        ));
                    }
                }
            }
        }
        unreachable!()
    }

    // Internal method to try a single send with timeout
    async fn try_send(&mut self, data: &[u8], timeout_duration: Duration) -> anyhow::Result<()> {
        let start_time = std::time::Instant::now();
        
        while start_time.elapsed() < timeout_duration {
            let (ack, _answer) = self.radio.send_packet_async(self.channel, self.address, data.to_vec()).await
                .map_err(|e| anyhow::anyhow!("Radio error during send: {}", e))?;

            if ack.received {
                return Ok(());
            }
            
            // Short delay before retry
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
        
        Err(anyhow::anyhow!("Timeout: No ACK received within {:?}", timeout_duration))
    }
}