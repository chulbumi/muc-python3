# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        mode = False
        msg = ''
        cnt = 0
        
        for c in ob.adultCH:
            if c['투명상태'] == 1:
                continue
            if c['이름'] != '' and c.state == ACTIVE:
                if mode and c['소속'] != ob['소속']:
                    continue
                nick = c['무림별호']
                
                if nick == '':
                    buf = '[[0;37m%s[0;37m]' % '무명객'
                else:
                    if c['성격'] == '정파':
                        buf = '[[1;32m%s[0;37m]' % nick
                    elif c['성격'] == '기인':
                        buf = '[[1;33m%s[0;37m]' % nick
                    elif c['성격'] == '선인':
                        buf = '[[1;36m%s[0;37m]' % nick
                    else:
                        buf = '<[1;31m%s[0;37m>' % nick
                    
                msg += '  %-26s %-10s' % (buf, c['이름'])
                cnt += 1
                if cnt % 3 == 0:
                    msg += '\r\n'
        if cnt % 3 == 0:
            msg = msg[:-2]
        ob.sendLine('┌─────────────────────────────────────┐')
        ob.sendLine('│[7m%-74s[0;37m│' % ' ◁     무       림       크       래       프       트      １-１     ▷');
        ob.sendLine('└─────────────────────────────────────┘')
        ob.sendLine(msg);
        ob.sendLine(' ──────────────────────────────────────')
        if mode:
            ob.sendLine(' ★ 총 %d명의 [1m【[36m%s[37m】[0;37m파 무림인이 활동하고 있습니다.' % (cnt, ob['소속']))
        else:
            ob.sendLine(' ★ 총 %d명의 무림인이 활동하고 있습니다.' % cnt)

